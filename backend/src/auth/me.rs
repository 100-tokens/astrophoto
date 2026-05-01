use axum::{Json, extract::State, response::IntoResponse};

use crate::AppError;
use crate::api_types::User;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    _state: State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let dto: User = user.into();
    Ok(Json(dto))
}
