//! Photo processing pipeline. Used by both the HTTP upload handler and
//! the seed binary. Synchronous (awaits each step). The HTTP handler
//! splits the synchronous insert (returns id quickly) from the
//! background `finalize` (EXIF + thumbs).

use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::photos::{exif, queries, thumbs};
use crate::storage::Storage;

const THUMB_SIZES: &[u32] = &[400, 1200];

/// Controls which fields `finalize` writes back after image processing.
#[derive(Clone, Copy, Debug)]
pub enum PipelineOptions {
    /// Initial upload — write all derived metadata (EXIF, width, height).
    Initial,
    /// Replace — skip writing user-controlled fields (target/caption/exif),
    /// only refresh width/height; drain pending S3 deletes on success.
    Replace,
}

/// Full synchronous pipeline: insert + finalize. Used by the seed
/// binary. The HTTP handler uses the (insert) + (background finalize)
/// pair instead.
#[allow(clippy::too_many_arguments)]
pub async fn process(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
    options: PipelineOptions,
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
    if let Err(e) = finalize(pool, storage, photo_id, bytes, options).await {
        let reason = format!("{e}");
        let _ = queries::mark_failed(pool, photo_id, &reason).await;
        return Err(e);
    }
    Ok(photo_id)
}

/// Just the EXIF parse + thumb generation steps. The HTTP handler runs
/// this in `tokio::spawn` after the original is uploaded synchronously.
pub async fn finalize(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    photo_id: Uuid,
    bytes: Bytes,
    options: PipelineOptions,
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

    let (exif_data, generated) = parsed;
    let (full_w, full_h) = generated
        .iter()
        .max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in generated {
        let key = format!("thumbs/{photo_id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, photo_id, thumb.size as i32, &key, len).await?;
    }

    match options {
        PipelineOptions::Initial => {
            queries::mark_ready(pool, photo_id, full_w, full_h, &exif_data).await?;
        }
        PipelineOptions::Replace => {
            // Mark ready first — the new master image is good (decode + thumbnail
            // generation succeeded). If the deferred S3 cleanup that follows hits
            // an error, the photo is still ready; the hourly purge worker
            // (jobs::purge_deletions::sweep_pending_deletes) will retry stale rows.
            queries::mark_ready_size_only(pool, photo_id, full_w, full_h).await?;

            // Best-effort drain of pending S3 deletes. Failures are logged but
            // not propagated — the photo is healthy and the worker will catch
            // anything left over after 7 days.
            match queries::pending_deletes_for(pool, photo_id).await {
                Ok(keys) if !keys.is_empty() => {
                    if let Err(e) = storage.delete_objects(&keys).await {
                        tracing::warn!(
                            photo_id=%photo_id, error=%e,
                            "replace drain: storage delete failed; purge worker will retry"
                        );
                    } else if let Err(e) = queries::drain_pending_deletes(pool, photo_id).await {
                        tracing::warn!(
                            photo_id=%photo_id, error=%e,
                            "replace drain: pending_deletes row removal failed"
                        );
                    }
                }
                Ok(_) => {} // empty list — nothing to drain
                Err(e) => tracing::warn!(
                    photo_id=%photo_id, error=%e,
                    "replace drain: pending_deletes_for query failed"
                ),
            }
        }
    }
    Ok(())
}
