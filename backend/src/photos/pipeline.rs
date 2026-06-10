//! Photo processing pipeline. Used by both the HTTP upload handler and
//! the seed binary. Synchronous (awaits each step). The HTTP handler
//! splits the synchronous insert (returns id quickly) from the
//! background `finalize` (EXIF + thumbs + display master + blurhash).

use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::photos::{exif, queries, thumbs};
use crate::storage::Storage;

const THUMB_SIZES: &[u32] = &[400, 1200];

const DISPLAY_MASTER_LONG_EDGE: u32 = 4096;
const DISPLAY_MASTER_QUALITY: u8 = 85;

/// Resize to at most `DISPLAY_MASTER_LONG_EDGE` on the long side, encode as
/// JPEG q85. ICC and EXIF metadata are stripped by the encode round-trip.
/// Takes the already-decoded image — the pipeline decodes the original
/// exactly once (decode dominates CPU for large originals).
fn derive_display_master_blocking(img: &image::DynamicImage) -> Result<Bytes, AppError> {
    let (w, h) = (img.width(), img.height());
    let scale = if w.max(h) > DISPLAY_MASTER_LONG_EDGE {
        DISPLAY_MASTER_LONG_EDGE as f32 / w.max(h) as f32
    } else {
        1.0
    };
    let target_w = (w as f32 * scale) as u32;
    let target_h = (h as f32 * scale) as u32;
    let resized_owned;
    let resized: &image::DynamicImage = if scale < 1.0 {
        resized_owned = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);
        &resized_owned
    } else {
        img
    };
    let mut out = Vec::with_capacity(256 * 1024);
    let mut enc =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, DISPLAY_MASTER_QUALITY);
    enc.encode_image(resized)
        .map_err(|e| AppError::Internal(format!("display encode: {e}")))?;
    Ok(Bytes::from(out))
}

/// Compute a blurhash string using 4×3 components from a 32×32 downsample.
fn derive_blurhash_blocking(img: &image::DynamicImage) -> Result<String, AppError> {
    let small = img.resize(32, 32, image::imageops::FilterType::Triangle);
    let rgba = small.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let pixels = rgba.into_raw();
    let hash = blurhash::encode(4, 3, w, h, &pixels)
        .map_err(|e| AppError::Internal(format!("blurhash: {e}")))?;
    Ok(hash)
}

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

/// EXIF parse + thumbnail generation + display-master derivation + blurhash.
/// The HTTP handler runs this in `tokio::spawn` after the original is
/// uploaded synchronously.
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
        // Decode ONCE and derive every artifact (thumbs, display master,
        // blurhash) from the same DynamicImage — decoding a 50–100 MP
        // original dominates this pipeline's CPU and used to run four
        // times per upload. The error stays Validation to match the
        // category the first thumb decode used to produce.
        let img = image::load_from_memory(&bytes_for_blocking)
            .map_err(|e| AppError::Validation(format!("decode: {e}")))?;
        let mut generated = Vec::with_capacity(THUMB_SIZES.len());
        for size in THUMB_SIZES {
            generated.push(thumbs::generate_blocking(&img, *size)?);
        }
        let display = derive_display_master_blocking(&img)?;
        let blurhash = derive_blurhash_blocking(&img)?;
        Ok::<_, AppError>((exif_data, generated, display, blurhash))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking join: {e}")))??;

    let (exif_data, generated, display_bytes, blurhash) = parsed;
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

    // Upload the display master and persist metadata before marking the photo
    // ready so readers never observe status='ready' with a null display_key.
    let display_key = format!("display/{photo_id}.jpg");
    storage
        .put(&display_key, "image/jpeg", display_bytes)
        .await?;
    queries::set_display_metadata(pool, photo_id, &display_key, &blurhash).await?;

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

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn display_master_clamps_long_edge() {
        let big = include_bytes!("../../tests/fixtures/wide_5000.jpg");
        let decoded = image::load_from_memory(big).unwrap();
        let out = derive_display_master_blocking(&decoded).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert!(img.width().max(img.height()) <= 4096);
    }
}
