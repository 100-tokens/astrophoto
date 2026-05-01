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

async fn resolve(state: &AppState, parts: &Parts) -> Result<Option<UserRow>, AppError> {
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
