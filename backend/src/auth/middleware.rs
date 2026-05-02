use axum::{
    extract::FromRequestParts,
    http::{header::COOKIE, request::Parts},
};

use crate::AppError;
use crate::auth::session;
use crate::http::AppState;
use crate::users::queries::{self, UserRow};

/// Holds the authenticated user, or `None` if no valid session.
pub struct CurrentUser(pub UserRow);
/// Holds an optional user; `None` when the request has no valid session cookie.
pub struct OptionalUser(pub Option<UserRow>);

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
