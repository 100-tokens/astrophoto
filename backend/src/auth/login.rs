use std::net::IpAddr;

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::User;
use crate::auth::{password, session};
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

    let stored = user.password_hash.clone().ok_or(AppError::Unauthorized)?;
    let ok = password::verify(body.password, stored).await?;
    if !ok {
        return Err(AppError::Unauthorized);
    }

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
