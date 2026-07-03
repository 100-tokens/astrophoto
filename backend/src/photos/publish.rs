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
    // Guarded UPDATE, not just the read-check above: a replace claim can
    // flip status to 'processing' between the read and the write, and a
    // concurrent publish must not shift published_at. 0 rows → the state
    // moved under us; report it like the read would have. (No
    // last_step='caption' write — that wizard step was removed in
    // 56acf4e; verify only ever writes 'verify'.)
    let published = sqlx::query!(
        "update photos set published_at = now()
          where id = $1 and status = 'ready' and published_at is null",
        id
    )
    .execute(&state.pool)
    .await?
    .rows_affected();
    if published == 0 {
        return Err(AppError::bad_request(
            "photo not ready: pipeline still processing or failed",
        ));
    }
    Ok(StatusCode::OK)
}
