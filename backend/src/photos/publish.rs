use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select owner_id, status, published_at from photos where id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::not_found("photo"))?;

    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }
    if row.published_at.is_some() {
        return Ok(StatusCode::OK); // idempotent no-op
    }
    if row.status != "ready" {
        return Err(AppError::bad_request(
            "photo not ready: pipeline still processing or failed",
        ));
    }
    sqlx::query!(
        "update photos set published_at = now(), last_step = 'caption' where id = $1",
        id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}
