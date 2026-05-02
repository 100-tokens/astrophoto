use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::{pipeline, queries};

const MAX_BYTES: usize = 50 * 1024 * 1024;
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select owner_id, status, storage_key from photos where id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::not_found("photo"))?;
    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }
    if row.status == "processing" {
        return Err(AppError::BadRequest("pipeline busy".into()));
    }

    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("upload.bin");
    let mut mime = String::from("application/octet-stream");
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        if field.name() == Some("file") {
            if let Some(n) = field.file_name() {
                filename = n.to_string();
            }
            if let Some(c) = field.content_type() {
                mime = c.to_string();
            }
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Validation(format!("read: {e}")))?;
            if data.len() > MAX_BYTES {
                return Err(AppError::Validation(format!(
                    "file too large: {} bytes (max {MAX_BYTES})",
                    data.len()
                )));
            }
            file_bytes = Some(data);
        }
    }
    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    // 1. Stash old master + thumb keys for deferred deletion.
    let mut to_stash = vec![row.storage_key.clone()];
    let old_thumb_keys: Vec<String> =
        sqlx::query_scalar!("select storage_key from thumbnails where photo_id = $1", id)
            .fetch_all(&state.pool)
            .await?;
    to_stash.extend(old_thumb_keys);
    queries::enqueue_pending_deletes(&state.pool, id, &to_stash).await?;

    // 2. Upload new master to a fresh key.
    let new_key = format!("originals/{}", Uuid::new_v4());
    state.storage.put(&new_key, &mime, bytes.clone()).await?;

    // 3. Atomically swap key + size + mime + replaced_at + status='processing'.
    queries::swap_storage_key_for_replace(
        &state.pool,
        id,
        &new_key,
        &filename,
        &mime,
        bytes.len() as i64,
    )
    .await?;

    // 4. DELETE old thumbnail rows (S3 keys already stashed).
    sqlx::query!("delete from thumbnails where photo_id = $1", id)
        .execute(&state.pool)
        .await?;

    // 5. Spawn pipeline with Replace options — drains pending deletes on success.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    tokio::spawn(async move {
        if let Err(e) = pipeline::finalize(
            &pool,
            storage,
            id,
            bytes,
            pipeline::PipelineOptions::Replace,
        )
        .await
        {
            let reason = format!("{e}");
            tracing::error!(photo_id=%id, error=%reason, "replace finalize failed");
            let _ = queries::mark_failed(&pool, id, &reason).await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}
