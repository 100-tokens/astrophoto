//! User avatar upload.
//!
//! Avatars follow the same presigned-PUT-direct-to-S3 pattern as photo
//! frames (so the large image body never traverses the SvelteKit reverse
//! proxy's `BODY_SIZE_LIMIT`), but they are NOT photos: no `photos` row,
//! no thumbnails / EXIF / blurhash. The processed avatar is written to
//! `display/<avatar_id>.jpg` — the exact key the CloudFront + Lambda@Edge
//! origin-request handler already serves for `/img/<uuid>` — so avatars
//! render through the CDN with zero infra change. See migration
//! `0030_add_user_avatar.sql`.
//!
//! Flow. `POST /api/me/avatar/init` validates size+mime, mints a fresh
//! `avatar_id`, and returns a presigned PUT to `avatar-uploads/<avatar_id>`.
//! The browser PUTs the raw image straight to S3. `POST
//! /api/me/avatar/finalize` then fetches that raw object, decodes it under
//! strict limits (anti-decompression-bomb), resizes to a small JPEG, writes
//! `display/<avatar_id>.jpg`, sets `users.avatar_id`, and deletes the temp
//! object plus the previous avatar's display object. `DELETE /api/me/avatar`
//! clears the avatar and deletes its display object.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

/// Max raw-upload size accepted by `init`. Avatars are tiny; this is a
/// generous ceiling that still keeps the presigned PUT cheap.
const AVATAR_MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Presigned PUT lifetime (same as photo uploads).
const PUT_TTL_SECS: u64 = 600;

/// Long-edge cap of the stored display master. The CDN crops to the small
/// circle sizes the UI requests (32–144 px) on the fly via `fit=cover`, so
/// there is no point storing anything large.
const AVATAR_LONG_EDGE: u32 = 512;
const AVATAR_QUALITY: u8 = 85;
/// Decompression-bomb guard: reject inputs that decode to absurd
/// dimensions, and cap the decoder's intermediate allocation. A 5 MB PNG
/// can otherwise inflate to many GB of RGBA in memory.
const AVATAR_MAX_DIM: u32 = 10_000;
const AVATAR_MAX_ALLOC: u64 = 256 * 1024 * 1024;

fn temp_key(avatar_id: Uuid) -> String {
    format!("avatar-uploads/{avatar_id}")
}

fn display_key(avatar_id: Uuid) -> String {
    format!("display/{avatar_id}.jpg")
}

#[derive(Deserialize)]
pub struct InitBody {
    pub size: u64,
    pub mime: String,
}

#[derive(Serialize)]
pub struct InitResponse {
    pub avatar_id: String,
    pub presigned_put_url: String,
}

pub async fn init(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<InitBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.size == 0 || body.size > AVATAR_MAX_BYTES {
        return Err(AppError::Validation(format!(
            "avatar must be 1..={AVATAR_MAX_BYTES} bytes"
        )));
    }
    // image 0.25 is built with jpeg + png decoders only (no webp), so the
    // finalize decode would fail on anything else; reject early.
    match body.mime.as_str() {
        "image/jpeg" | "image/png" => {}
        other => return Err(AppError::UnsupportedFormat(other.to_string())),
    }

    let avatar_id = Uuid::new_v4();
    let url = state
        .storage
        .presigned_put(&temp_key(avatar_id), &body.mime, body.size, PUT_TTL_SECS)
        .await?;

    // Note: nothing is written to the DB here. `users.avatar_id` is only set
    // once finalize has the bytes and produced a display master, so an
    // abandoned init leaves no trace beyond an orphan temp object (which the
    // next finalize / a manual sweep can remove — it is never referenced).
    let _ = user;
    Ok(Json(InitResponse {
        avatar_id: avatar_id.to_string(),
        presigned_put_url: url,
    }))
}

#[derive(Deserialize)]
pub struct FinalizeBody {
    pub avatar_id: Uuid,
}

#[derive(Serialize)]
pub struct FinalizeResponse {
    pub avatar_id: String,
}

pub async fn finalize(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<FinalizeBody>,
) -> Result<impl IntoResponse, AppError> {
    let avatar_id = body.avatar_id;
    let tmp = temp_key(avatar_id);

    // The raw object the browser PUT directly to S3. None means the PUT never
    // landed (or the presigned URL expired) — caller must redo init + PUT.
    let raw = state.storage.get(&tmp).await?.ok_or_else(|| {
        AppError::PendingFinalizeStuck("no uploaded avatar — did the PUT succeed?".into())
    })?;

    // Decode + resize + re-encode is CPU-bound: never on the async runtime.
    let master = tokio::task::spawn_blocking(move || derive_avatar_master_blocking(&raw))
        .await
        .map_err(|e| AppError::Internal(format!("avatar join: {e}")))??;

    // Write the CDN-served display master.
    state
        .storage
        .put(&display_key(avatar_id), "image/jpeg", master)
        .await?;

    // Flip the pointer, capturing the previous avatar so we can GC its object.
    let previous: Option<Uuid> =
        sqlx::query_scalar!("select avatar_id from users where id = $1", user.id)
            .fetch_one(&state.pool)
            .await?;
    sqlx::query!(
        "update users set avatar_id = $1 where id = $2",
        avatar_id,
        user.id
    )
    .execute(&state.pool)
    .await?;

    // Best-effort cleanup: drop the temp upload and the now-unreferenced old
    // display master. Failures here are non-fatal (the avatar is already live).
    let _ = state.storage.delete(&tmp).await;
    if let Some(old) = previous
        && old != avatar_id
    {
        let _ = state.storage.delete(&display_key(old)).await;
    }

    Ok(Json(FinalizeResponse {
        avatar_id: avatar_id.to_string(),
    }))
}

pub async fn clear(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let previous: Option<Uuid> =
        sqlx::query_scalar!("select avatar_id from users where id = $1", user.id)
            .fetch_one(&state.pool)
            .await?;
    sqlx::query!("update users set avatar_id = null where id = $1", user.id)
        .execute(&state.pool)
        .await?;
    if let Some(old) = previous {
        let _ = state.storage.delete(&display_key(old)).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Decode `bytes` under strict limits, resize so the long edge is at most
/// `AVATAR_LONG_EDGE`, and encode as JPEG q85. The decode round-trip strips
/// any EXIF/ICC metadata. Returns `Validation` for anything that is not a
/// decodable image within the bomb-guard limits.
fn derive_avatar_master_blocking(bytes: &[u8]) -> Result<Bytes, AppError> {
    use std::io::Cursor;

    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::Validation(format!("avatar format: {e}")))?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(AVATAR_MAX_DIM);
    limits.max_image_height = Some(AVATAR_MAX_DIM);
    limits.max_alloc = Some(AVATAR_MAX_ALLOC);
    reader.limits(limits);

    let img = reader
        .decode()
        .map_err(|e| AppError::Validation(format!("avatar decode: {e}")))?;

    let (w, h) = (img.width(), img.height());
    let resized = if w.max(h) > AVATAR_LONG_EDGE {
        let scale = AVATAR_LONG_EDGE as f32 / w.max(h) as f32;
        let tw = ((w as f32 * scale) as u32).max(1);
        let th = ((h as f32 * scale) as u32).max(1);
        img.resize(tw, th, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // JPEG has no alpha channel — `encode_image` rejects an Rgba8 image. PNG
    // avatars (logos, exports) are frequently transparent, so flatten onto a
    // white background before encoding rather than failing or going black.
    let rgb = flatten_onto_white(&resized);

    let mut out = Vec::with_capacity(64 * 1024);
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, AVATAR_QUALITY);
    enc.encode_image(&rgb)
        .map_err(|e| AppError::Internal(format!("avatar encode: {e}")))?;
    Ok(Bytes::from(out))
}

/// Composite `img` onto an opaque white background, dropping the alpha
/// channel. Opaque images pass through unchanged (alpha 255 → source pixel);
/// this is a cheap no-op copy for the common JPEG case and the correct
/// behaviour for transparent PNGs.
fn flatten_onto_white(img: &image::DynamicImage) -> image::RgbImage {
    use image::{Rgb, RgbImage};

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = RgbImage::new(w, h);
    for (x, y, px) in rgba.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        let a = u16::from(a);
        // out = src*a + white*(255-a), rounded.
        let blend = |c: u8| (((u16::from(c) * a) + 255 * (255 - a) + 127) / 255) as u8;
        out.put_pixel(x, y, Rgb([blend(r), blend(g), blend(b)]));
    }
    out
}
