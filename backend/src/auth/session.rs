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

pub const COOKIE_NAME: &str = "__Host-session";
pub const SESSION_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct SessionRow {
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
pub fn cookie_header(value: &str, secure: bool, max_age_days: i64) -> String {
    let secure_part = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age={max_age}",
        name = COOKIE_NAME,
        value = value,
        secure = secure_part,
        max_age = max_age_days * 86_400
    )
}

/// Build a `Set-Cookie` header value that clears the session cookie.
pub fn clear_cookie_header(secure: bool) -> String {
    let secure_part = if secure { "; Secure" } else { "" };
    format!(
        "{name}=; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age=0",
        name = COOKIE_NAME,
        secure = secure_part,
    )
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
        assert!(h.starts_with("__Host-session=abc"));
        assert!(h.contains("HttpOnly"));
        assert!(h.contains("Secure"));
        assert!(h.contains("SameSite=Lax"));
        assert!(h.contains("Path=/"));
        assert!(h.contains("Max-Age=2592000"));
    }

    #[test]
    fn cookie_header_drops_secure_when_disabled() {
        let h = cookie_header("abc", false, 1);
        assert!(!h.contains("Secure"));
        assert!(h.contains("Max-Age=86400"));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode("not!valid!base64").is_none());
    }
}
