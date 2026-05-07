use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let row = sqlx::query!(
        "select owner_id, storage_key, status, published_at from photos where id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::not_found("photo"))?;

    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let cancellable =
        row.published_at.is_none() && (row.status == "pending" || row.status == "processing");
    if !cancellable {
        return Err(AppError::Conflict("photo is not cancellable".into()));
    }

    sqlx::query!("delete from photos where id = $1", id)
        .execute(&state.pool)
        .await?;

    // Best-effort S3 cleanup. If this fails we log but return 204 — the DB row
    // is gone so the upload is functionally cancelled; orphan S3 objects are
    // recoverable via a sweep if one is added later.
    if let Err(e) = state.storage.delete_objects(&[row.storage_key]).await {
        tracing::warn!(photo_id=%id, error=%e, "upload_cancel: S3 cleanup failed");
    }

    Ok(StatusCode::NO_CONTENT)
}
