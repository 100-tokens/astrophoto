use axum::{
    extract::FromRequestParts,
    http::{header::COOKIE, request::Parts},
};

use crate::AppError;
use crate::auth::session;
use crate::http::AppState;
use crate::users::queries::{self, UserRow};

/// Marker stashed in request extensions when authentication came from a
/// Bearer PAT instead of a session cookie. `AdminUser` and the token-
/// management endpoints reject such requests.
#[derive(Clone, Copy)]
pub struct TokenAuth;

/// Holds the authenticated user, or `None` if no valid session.
pub struct CurrentUser(pub UserRow);
/// Holds an optional user; `None` when the request has no valid session cookie.
pub struct OptionalUser(pub Option<UserRow>);

/// Guards `/api/admin/*` routes: yields the authenticated user only when they
/// are a super-admin. Self-contained (resolves the session itself, no
/// ordering dependency): `401 Unauthorized` with no valid session, `403
/// Forbidden` when authenticated but `is_admin` is false.
pub struct AdminUser(pub UserRow);

/// Rejects requests authenticated via a Bearer PAT. Add to handlers
/// that must stay browser-session-only: account control (password /
/// email change, deletion) and PAT management itself. A stolen token
/// must never be able to escalate into account takeover.
///
/// **Ordering invariant:** must appear AFTER [`CurrentUser`] in the
/// handler signature — `CurrentUser`'s `resolve()` populates the
/// `TokenAuth` extension this extractor inspects.
pub struct SessionOnly;

/// Session id of the currently-authenticated request.
///
/// **Ordering invariant:** This extractor must appear AFTER
/// [`CurrentUser`] in any handler signature. `CurrentUser`'s extraction
/// runs the session lookup that populates the request extensions; if
/// `CurrentSessionId` runs first, it returns `Unauthorized("no_session")`
/// even for valid sessions.
///
/// Wrong:
/// ```ignore
/// async fn h(CurrentSessionId(_): CurrentSessionId, CurrentUser(_): CurrentUser) { ... }
/// ```
/// Right:
/// ```ignore
/// async fn h(CurrentUser(_): CurrentUser, CurrentSessionId(_): CurrentSessionId) { ... }
/// ```
#[derive(Clone)]
pub struct CurrentSessionId(pub Vec<u8>);

async fn resolve(state: &AppState, parts: &mut Parts) -> Result<Option<UserRow>, AppError> {
    use crate::auth::tokens::{TOKEN_PREFIX, hash_secret};
    use axum::http::header::AUTHORIZATION;

    // Bearer PAT branch: resolve the token to its user before falling
    // through to the cookie path. Only our PAT format short-circuits
    // here; anything else falls through so future schemes stay possible.
    if let Some(value) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        && value.starts_with(TOKEN_PREFIX)
    {
        let hash = hash_secret(value);
        let Some(row) = sqlx::query!(
            r#"select id, user_id from api_tokens
                where token_hash = $1 and revoked_at is null"#,
            hash
        )
        .fetch_optional(&state.pool)
        .await?
        else {
            return Ok(None);
        };

        parts.extensions.insert(TokenAuth);

        // Throttled last_used_at, same contract as sessions below.
        if let Err(e) = sqlx::query!(
            "update api_tokens set last_used_at = now() \
             where id = $1 and (last_used_at is null \
                or last_used_at < now() - interval '5 minutes')",
            row.id
        )
        .execute(&state.pool)
        .await
        {
            tracing::warn!(error = %e, "api_token last_used_at update failed");
        }

        return queries::find_by_id(&state.pool, row.user_id).await;
    }

    let Some(cookie_header) = parts.headers.get(COOKIE) else {
        return Ok(None);
    };
    let Ok(cookie_str) = cookie_header.to_str() else {
        return Ok(None);
    };
    // Accept either `__Host-session=` (HTTPS prod) or `session=` (HTTP dev).
    let Some(value) = cookie_str.split(';').map(str::trim).find_map(|kv| {
        session::COOKIE_NAMES
            .iter()
            .find_map(|name| kv.strip_prefix(&format!("{name}=")))
    }) else {
        return Ok(None);
    };
    let Some(s) = session::lookup(&state.pool, value).await? else {
        return Ok(None);
    };

    // Stash the session id so `CurrentSessionId` extractor can retrieve it.
    parts.extensions.insert(CurrentSessionId(s.id.clone()));

    // Throttled last_used_at update: only fires if row is older than 5 min.
    // Errors swallowed — one DB hiccup must not break authenticated requests.
    if let Err(e) = sqlx::query!(
        "update sessions set last_used_at = now() \
         where id = $1 and last_used_at < now() - interval '5 minutes'",
        s.id
    )
    .execute(&state.pool)
    .await
    {
        tracing::warn!(error = %e, "last_used_at throttled update failed");
    }

    let user = queries::find_by_id(&state.pool, s.user_id).await?;
    Ok(user)
}

#[axum::async_trait]
impl FromRequestParts<AppState> for OptionalUser {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalUser(resolve(state, parts).await?))
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match resolve(state, parts).await? {
            Some(u) => Ok(CurrentUser(u)),
            None => Err(AppError::Unauthorized),
        }
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let resolved = resolve(state, parts).await?;
        // A Bearer PAT must never satisfy an admin route, even for an
        // admin user — reject before the is_admin check.
        if parts.extensions.get::<TokenAuth>().is_some() {
            return Err(AppError::Forbidden);
        }
        match resolved {
            Some(u) if u.is_admin => Ok(AdminUser(u)),
            Some(_) => Err(AppError::Forbidden),
            None => Err(AppError::Unauthorized),
        }
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for SessionOnly {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if parts.extensions.get::<TokenAuth>().is_some() {
            return Err(AppError::Forbidden);
        }
        Ok(SessionOnly)
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for CurrentSessionId {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // `CurrentUser` must be extracted first — it stashes the id in extensions.
        parts
            .extensions
            .get::<CurrentSessionId>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
