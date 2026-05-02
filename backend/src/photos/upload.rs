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
use crate::photos::{exif, queries, thumbs};
use crate::storage::Storage;

const MAX_BYTES: usize = 50 * 1024 * 1024; // 50 MB
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];
const THUMB_SIZES: &[u32] = &[400, 1200];

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
            _ => {} // ignore unknown fields
        }
    }

    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    state
        .storage
        .put(&storage_key, &mime, bytes.clone())
        .await?;

    let id = queries::insert_processing(
        &state.pool,
        user.id,
        &storage_key,
        &filename,
        bytes.len() as i64,
        &mime,
        target.as_deref(),
        caption.as_deref(),
    )
    .await?;

    // Spawn background processing.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    let bytes_for_proc = bytes.clone();
    tokio::spawn(async move {
        if let Err(e) = process_photo(&pool, storage, id, bytes_for_proc).await {
            tracing::error!(photo_id=%id, error=%e, "photo processing failed");
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

async fn process_photo(
    pool: &sqlx::PgPool,
    storage: Arc<dyn Storage>,
    id: Uuid,
    bytes: Bytes,
) -> Result<(), AppError> {
    let bytes_for_blocking = bytes.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        let exif_data = exif::parse_blocking(&bytes_for_blocking)?;
        let mut generated = Vec::with_capacity(THUMB_SIZES.len());
        for size in THUMB_SIZES {
            generated.push(thumbs::generate_blocking(&bytes_for_blocking, *size)?);
        }
        Ok::<_, AppError>((exif_data, generated))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking join: {e}")))??;

    let (exif_data, thumbs_out) = parsed;

    // Pick the largest as the canonical width/height (input image size,
    // since smaller-than-max thumbnails preserve original dimensions).
    let (full_w, full_h) = thumbs_out
        .iter()
        .max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in thumbs_out {
        let key = format!("thumbs/{id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, id, thumb.size as i32, &key, len).await?;
    }

    queries::mark_ready(pool, id, full_w, full_h, &exif_data).await?;
    Ok(())
}
