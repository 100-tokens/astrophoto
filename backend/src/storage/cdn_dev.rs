//! Dev-only CDN handler: serves on-the-fly JPEG resizes from the display master
//! stored at `display/<id>.jpg` in object storage.
//!
//! Mounted only when `config.cdn_base_url` points back at this process
//! (contains "localhost" or "127.0.0.1"). In production, CloudFront sits in
//! front and this route is not registered.
//!
//! Output is always JPEG regardless of the `fm` query param (format conversion
//! is handled by CloudFront/Lambda@Edge in production).

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::Response,
};
use bytes::Bytes;
use image::{ImageFormat, imageops::FilterType};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::http::AppState;

/// Resize-target bounds. The route is unauthenticated and (via
/// `APP_CDN_LOCAL_FALLBACK`) can be live on staging/prod, so unbounded
/// `w`/`h` would let one GET allocate gigabytes (`resize_to_fill(50000,
/// 50000)` ≈ 7.5 GB of RGB). 4096 matches the display-master ceiling —
/// nothing larger can ever be useful. Out-of-range values are clamped,
/// not rejected, so existing cached URLs keep working.
const MIN_DIM: u32 = 16;
const MAX_DIM: u32 = 4096;

#[derive(Deserialize, Default)]
pub struct Q {
    /// Target width in pixels.
    pub w: Option<u32>,
    /// Target height in pixels.
    pub h: Option<u32>,
    /// Resize mode: "contain" (letterbox) or anything else → "cover" (fill+crop).
    pub fit: Option<String>,
    /// JPEG output quality, 1–100. Defaults to 85.
    pub q: Option<u8>,
    /// Output format hint (ignored in dev — always JPEG).
    pub fm: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<Q>,
) -> Result<Response, AppError> {
    let key = format!("display/{id}.jpg");
    let bytes = state
        .storage
        .get(&key)
        .await?
        .ok_or_else(|| AppError::NotFound("display master".into()))?;

    let resized = tokio::task::spawn_blocking(move || -> Result<Bytes, AppError> {
        let img = image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg)
            .map_err(|e| AppError::Internal(format!("decode: {e}")))?;

        let target_w = q.w.unwrap_or_else(|| img.width()).clamp(MIN_DIM, MAX_DIM);
        let target_h = q.h.unwrap_or_else(|| img.height()).clamp(MIN_DIM, MAX_DIM);
        let fit = q.fit.as_deref().unwrap_or("cover");

        let resized = match fit {
            "contain" => img.resize(target_w, target_h, FilterType::Lanczos3),
            _ => img.resize_to_fill(target_w, target_h, FilterType::Lanczos3),
        };

        let mut out = Vec::with_capacity(64 * 1024);
        let quality = q.q.unwrap_or(85).clamp(1, 100);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        enc.encode_image(&resized)
            .map_err(|e| AppError::Internal(format!("encode: {e}")))?;

        Ok(Bytes::from(out))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking: {e}")))??;

    let resp = Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "image/jpeg")
        .header(CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(resized))
        .map_err(|e| AppError::Internal(format!("build resp: {e}")))?;

    Ok(resp)
}
