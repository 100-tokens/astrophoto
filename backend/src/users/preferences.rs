use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::Preferences;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Preferences>, AppError> {
    let row = sqlx::query!(
        "select theme, density from users where id = $1", user.id
    ).fetch_one(&state.pool).await?;
    Ok(Json(Preferences { theme: row.theme, density: row.density }))
}

#[derive(Deserialize)]
pub struct PutBody {
    pub theme: Option<String>,
    pub density: Option<String>,
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(t) = &body.theme {
        if t != "dark" && t != "light" {
            return Err(AppError::bad_request("invalid_theme"));
        }
    }
    if let Some(d) = &body.density {
        if d != "work" && d != "data" {
            return Err(AppError::bad_request("invalid_density"));
        }
    }
    sqlx::query!(
        "update users
            set theme = coalesce($1, theme),
                density = coalesce($2, density)
          where id = $3",
        body.theme, body.density, user.id
    ).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
