use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::{pipeline, queries};
use crate::storage::Storage;

const MAX_BYTES: usize = 50 * 1024 * 1024;
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];

#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub status: String,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("upload.bin");
    let mut mime = String::from("application/octet-stream");
    let mut target: Option<String> = None;
    let mut caption: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                if let Some(ct) = field.content_type() {
                    mime = ct.to_string();
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
            Some("target") => {
                target = field.text().await.ok().filter(|s| !s.is_empty());
            }
            Some("caption") => {
                caption = field.text().await.ok().filter(|s| !s.is_empty());
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    // Synchronous part: upload original, insert row → returns the id.
    let id = quickstart(
        &state.pool,
        &state.storage,
        user.id,
        &filename,
        &mime,
        target.as_deref(),
        caption.as_deref(),
        bytes.clone(),
    )
    .await?;

    // Background: EXIF + thumbnails.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    let bytes_clone = bytes;
    tokio::spawn(async move {
        if let Err(e) = pipeline::finalize(&pool, storage, id, bytes_clone).await {
            tracing::error!(photo_id=%id, error=%e, "photo finalize failed");
            let _ = queries::mark_failed(&pool, id).await;
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadResponse {
            id: id.to_string(),
            status: "processing".into(),
        }),
    ))
}

/// Synchronous insert path used by the HTTP handler. Returns the DB-assigned id
/// so the caller can respond 202 with `{id, status}` immediately.
async fn quickstart(
    pool: &sqlx::PgPool,
    storage: &Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
) -> Result<Uuid, AppError> {
    let storage_key_prefix = Uuid::new_v4();
    let storage_key = format!("originals/{storage_key_prefix}");
    storage.put(&storage_key, mime, bytes.clone()).await?;
    let photo_id = queries::insert_processing(
        pool,
        owner_id,
        &storage_key,
        original_name,
        bytes.len() as i64,
        mime,
        target,
        caption,
    )
    .await?;
    Ok(photo_id)
}
