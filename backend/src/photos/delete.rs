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
    let row = sqlx::query!("select owner_id, storage_key from photos where id = $1", id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::not_found("photo"))?;

    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    // Collect master + thumbnail S3 keys BEFORE the delete so the photos row
    // (and its CASCADE-deleted thumbnails) still exist when we query them.
    let mut to_delete = vec![row.storage_key];
    let thumb_keys: Vec<String> =
        sqlx::query_scalar!("select storage_key from thumbnails where photo_id = $1", id)
            .fetch_all(&state.pool)
            .await?;
    to_delete.extend(thumb_keys);

    // Delete the photos row. CASCADE removes thumbnails, appreciations, comments,
    // and photo_pending_deletes for this photo.
    sqlx::query!("delete from photos where id = $1", id)
        .execute(&state.pool)
        .await?;

    // Best-effort S3 cleanup. If this fails we log but return 204 — the DB row
    // is gone so the photo is functionally deleted; orphan S3 objects are
    // recoverable via a sweep if one is added later.
    if let Err(e) = state.storage.delete_objects(&to_delete).await {
        tracing::warn!(photo_id=%id, error=%e, "delete: S3 cleanup failed");
    }

    Ok(StatusCode::NO_CONTENT)
}
