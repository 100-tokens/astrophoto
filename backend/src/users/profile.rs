use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::Profile;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Profile>, AppError> {
    let row = sqlx::query!("select display_name from users where id = $1", user.id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(Profile {
        display_name: row.display_name,
    }))
}

#[derive(Deserialize)]
pub struct PutBody {
    pub display_name: Option<String>,
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(name) = body.display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 60 {
            return Err(AppError::bad_request("invalid_display_name"));
        }
        sqlx::query!(
            "update users set display_name = $1 where id = $2",
            trimmed,
            user.id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
