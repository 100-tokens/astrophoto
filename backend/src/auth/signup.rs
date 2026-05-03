use std::net::IpAddr;

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::Deserialize;
use validator::Validate;

use crate::AppError;
use crate::api_types::User;
use crate::auth::{password, session};
use crate::users::queries;

#[derive(Deserialize, Validate)]
pub struct SignupBody {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 10, max = 200))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub handle: String,
}

#[allow(clippy::unwrap_used)]
pub async fn handler(
    State(state): State<crate::http::AppState>,
    headers: HeaderMap,
    Json(body): Json<SignupBody>,
) -> Result<impl IntoResponse, AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    crate::auth::handle::validate(&body.handle).map_err(|e| AppError::Validation(e.to_string()))?;

    let hash = password::hash(body.password).await?;
    let user = queries::create_with_password(
        &state.pool,
        &body.email,
        &body.handle,
        &body.display_name,
        &hash,
    )
    .await?;

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
    let mut response = (axum::http::StatusCode::CREATED, Json(user_dto)).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    Ok(response)
}
