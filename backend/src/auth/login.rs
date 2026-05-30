use std::net::IpAddr;

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::User;
use crate::auth::{login_throttle, password, session};
use crate::users::queries;

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[allow(clippy::unwrap_used)]
pub async fn handler(
    State(state): State<crate::http::AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = queries::find_by_email(&state.pool, &body.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Per-account brute-force throttle: a locked account short-circuits to a
    // generic 401 (uniform with a wrong password, and skipping Argon2 keeps the
    // timing the same as a non-existent email) BEFORE any expensive work. This
    // also means no failure is recorded while locked, so the lock duration is
    // fixed regardless of how long the attack continues. See `login_throttle`.
    if login_throttle::is_locked(&state.pool, user.id).await? {
        return Err(AppError::Unauthorized);
    }

    let stored = user.password_hash.clone().ok_or(AppError::Unauthorized)?;
    let ok = password::verify(body.password, stored).await?;
    if !ok {
        login_throttle::record_failure(&state.pool, user.id).await?;
        return Err(AppError::Unauthorized);
    }
    // Correct password: clear the throttle so a legitimate user who finally
    // remembers their password starts fresh. (This resets the counter; it does
    // not let a correct password bypass an *active* lock — that path returned
    // above without ever reaching the verify.)
    login_throttle::clear(&state.pool, user.id).await?;

    if user.email_verified_at.is_none() {
        return Err(AppError::Forbidden);
    }

    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let ip: Option<IpAddr> = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok());
    let cookie_value = session::create(&state.pool, user.id, ua, ip).await?;
    let cookie = session::cookie_header(
        &cookie_value,
        state.config.session_secure,
        session::SESSION_DAYS,
    );

    let user_dto: User = user.into();
    let mut response = Json(user_dto).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    Ok(response)
}
