//! Server-side session tokens stored in `sessions` table.
//!
//! Token: 32 random bytes generated with `OsRng`, stored as `bytea`. The same
//! bytes (base64url, no padding) form the cookie value sent to the browser.
//! Sessions live for 30 days from creation.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::{RngCore, rngs::OsRng};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;

pub const SESSION_DAYS: i64 = 30;

/// Cookie name. The `__Host-` prefix is browser-enforced to require `Secure`
/// (HTTPS only) and forbids `Domain`. In dev over plain HTTP the prefix would
/// cause the browser to silently reject the cookie, so we drop it. The Set-Cookie
/// header construction must use whichever name `cookie_name(secure)` returns.
pub fn cookie_name(secure: bool) -> &'static str {
    if secure { "__Host-session" } else { "session" }
}

/// Both names a request might carry — used on the read path so a server that
/// previously issued one prefix can still resolve sessions after a config flip.
pub const COOKIE_NAMES: &[&str] = &["__Host-session", "session"];

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: Vec<u8>,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Generate a new random 32-byte session token.
pub fn new_token() -> Vec<u8> {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf.to_vec()
}

/// Encode a token as base64url (no padding) for cookie transport.
pub fn encode(token: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(token)
}

/// Decode a base64url cookie value back to bytes. None if invalid.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(s).ok()
}

/// Insert a session row, returning the encoded cookie value.
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    user_agent: Option<&str>,
    ip: Option<std::net::IpAddr>,
) -> Result<String, AppError> {
    let token = new_token();
    let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
    sqlx::query!(
        r#"
        insert into sessions (id, user_id, expires_at, user_agent, ip)
        values ($1, $2, $3, $4, $5)
        "#,
        token,
        user_id,
        expires_at,
        user_agent,
        ip.map(sqlx::types::ipnetwork::IpNetwork::from),
    )
    .execute(pool)
    .await?;
    Ok(encode(&token))
}

/// Look up an active session by encoded cookie. Returns None when:
/// missing, malformed, expired, or revoked.
pub async fn lookup(pool: &PgPool, cookie: &str) -> Result<Option<SessionRow>, AppError> {
    let Some(token) = decode(cookie) else {
        return Ok(None);
    };
    let row = sqlx::query!(
        r#"
        select user_id, expires_at
        from sessions
        where id = $1 and expires_at > now()
        "#,
        token
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| SessionRow {
        id: token.clone(),
        user_id: r.user_id,
        expires_at: r.expires_at,
    }))
}

/// Delete a session by encoded cookie. Idempotent.
pub async fn delete(pool: &PgPool, cookie: &str) -> Result<(), AppError> {
    let Some(token) = decode(cookie) else {
        return Ok(());
    };
    sqlx::query!("delete from sessions where id = $1", token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Build the `Set-Cookie` header value for a fresh session token.
/// Caller decides Secure flag based on config.session_secure.
///
/// SameSite policy follows the deployment shape:
///
/// - `secure=true` (prod / staging over HTTPS): `SameSite=None` so the
///   session cookie is sent on cross-origin browser→backend fetches.
///   The frontend and backend live on different sibling subdomains
///   (e.g. `astrophoto-staging-web-*.koyeb.app` and
///   `astrophoto-staging-*.koyeb.app`) with no shared parent, so any
///   client-side fetch from the SvelteKit page to the API is
///   technically cross-site. `Lax` blocks those even with
///   `credentials: 'include'`. Cross-site exposure is bounded by the
///   single-origin CORS allow-list on the API.
/// - `secure=false` (dev over HTTP): `SameSite=Lax`. `SameSite=None`
///   requires `Secure`, which we cannot set on plain HTTP, so we keep
///   the historical `Lax` behaviour. Dev runs on `localhost` for both
///   frontend and backend, so cross-site is moot anyway.
pub fn cookie_header(value: &str, secure: bool, max_age_days: i64) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    let same_site = if secure { "None" } else { "Lax" };
    let name = cookie_name(secure);
    let max_age = max_age_days * 86_400;
    format!(
        "{name}={value}; HttpOnly{secure_attr}; SameSite={same_site}; Path=/; Max-Age={max_age}"
    )
}

/// Build a `Set-Cookie` header value that clears the session cookie.
/// SameSite policy mirrors `cookie_header` so the clear directive lands
/// on the same cookie the browser holds.
pub fn clear_cookie_header(secure: bool) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    let same_site = if secure { "None" } else { "Lax" };
    let name = cookie_name(secure);
    format!("{name}=; HttpOnly{secure_attr}; SameSite={same_site}; Path=/; Max-Age=0")
}

/// Create a fresh session for `user_id` and return the full `Set-Cookie`
/// header value string. Used by login, password-reset confirm, and
/// password-change to ensure identical cookie construction everywhere.
///
/// No user-agent or IP is recorded because this helper is called from paths
/// that may not have access to the request headers (e.g. reset-confirm, which
/// is a JSON body POST — the UA and IP matter less than the fresh token).
/// If you need them in future, add optional params here.
pub async fn create_session(state: &AppState, user_id: Uuid) -> Result<String, AppError> {
    let cookie_value = create(&state.pool, user_id, None, None).await?;
    Ok(cookie_header(
        &cookie_value,
        state.config.session_secure,
        SESSION_DAYS,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trips_through_base64url() {
        let t = new_token();
        let s = encode(&t);
        assert_eq!(decode(&s).unwrap(), t);
    }

    #[test]
    fn cookie_header_basic_attrs() {
        let h = cookie_header("abc", true, 30);
        assert!(h.starts_with("__Host-session=abc"), "got: {h}");
        assert!(h.contains("HttpOnly"));
        assert!(h.contains("Secure"));
        // SameSite=None is required so the cross-origin frontend in
        // staging/prod can submit the cookie on browser→backend fetches.
        assert!(h.contains("SameSite=None"), "got: {h}");
        assert!(!h.contains("SameSite=Lax"), "got: {h}");
        assert!(h.contains("Path=/"));
        assert!(h.contains("Max-Age=2592000"));
    }

    #[test]
    fn cookie_header_drops_secure_when_disabled() {
        let h = cookie_header("abc", false, 1);
        // Without HTTPS we drop both the Secure attribute and the __Host- prefix.
        assert!(h.starts_with("session=abc"), "got: {h}");
        assert!(!h.contains("__Host-"));
        assert!(!h.contains("Secure"));
        // Dev keeps SameSite=Lax — None requires Secure (which we drop on HTTP).
        assert!(h.contains("SameSite=Lax"), "got: {h}");
        assert!(h.contains("Max-Age=86400"));
    }

    #[test]
    fn clear_cookie_header_uses_same_attrs_as_set() {
        let secure = clear_cookie_header(true);
        assert!(secure.contains("Secure"), "got: {secure}");
        assert!(secure.contains("SameSite=None"), "got: {secure}");
        assert!(secure.contains("Max-Age=0"));

        let dev = clear_cookie_header(false);
        assert!(!dev.contains("Secure"), "got: {dev}");
        assert!(dev.contains("SameSite=Lax"), "got: {dev}");
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode("not!valid!base64").is_none());
    }
}
