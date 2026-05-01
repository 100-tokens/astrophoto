use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::Deserialize;

use crate::api_types::User;
use crate::auth::{password, session};
use crate::users::queries;
use crate::AppError;

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

    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let cookie_value = session::create(&state.pool, user.id, ua, None).await?;
    let cookie =
        session::cookie_header(&cookie_value, state.config.session_secure, session::SESSION_DAYS);

    let user_dto: User = user.into();
    let mut response = Json(user_dto).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    Ok(response)
}
