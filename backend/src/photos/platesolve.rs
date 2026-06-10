//! HTTP client for the plate-solve service
//! (xisf-rs-platesolve-server at `platesolve.astrophoto.pics`).
//!
//! # Scope
//!
//! This module ships the typed client + the persistence /
//! concurrency primitives used by the side-channel upload handler
//! in [`crate::photos::platesolve_upload`]. XISF is accepted only
//! via that endpoint — it is NOT a member of the standard upload
//! pipeline's mime allowlist. See `docs/platesolve-integration.md`
//! for the strategy choice (v1 = "Strategy A", side-channel).
//!
//! - [`PlatesolveClient`] — a `reqwest`-backed client with API-key
//!   auth, timeouts, and structured error mapping.
//! - [`PlatesolveResult`] — the parsed response shape, mirroring
//!   the server's `SolveResp`.
//! - [`save_result`] — writes the solve outcome onto a `photos` row
//!   (RA/Dec → existing columns; ancillary telemetry → the
//!   `platesolve_*` columns added in migration 0021).
//! - [`try_claim`] / [`mark_aborted_if_solving`] —
//!   atomic in-progress sentinel management used by the spawned
//!   background task.
//!
//! # Configuration
//!
//! Two env vars (see [`crate::config::Config`]):
//! - `APP_PLATESOLVE_BASE_URL` — `https://platesolve.astrophoto.pics` in
//!   prod; unset disables the feature.
//! - `APP_PLATESOLVE_API_KEY` — secret bearer token; required if
//!   the base URL is set.
//!
//! # Error semantics
//!
//! Every variant of [`PlatesolveError`] maps 1:1 to a documented
//! status the server returns (see
//! `xisf-rs/docs/platesolve-service-spec.md`). Callers can branch on
//! the variant to decide whether to retry (rate-limited /
//! service-unavailable → yes, after the `retry_after_secs` hint),
//! ask the user for more input (no-hint → "please fill RA/Dec"), or
//! treat the file as unsolvable (solve-failed / unsupported-media).

use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use reqwest::{Client, StatusCode, multipart};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use tracing::{info, warn};
use ts_rs::TS;
use uuid::Uuid;

use crate::config::Config;
use crate::error::AppError;

/// Hard cap on the size of an XISF body the side-channel
/// `/api/photos/:id/platesolve` endpoint will accept. Sized for the
/// Koyeb Nano/Micro tier (≤512 MB RSS) with concurrency bounded to 1
/// in `http::AppState::platesolve_permits`. The upstream service caps
/// at 256 MB (see `xisf-rs/docs/platesolve-service-spec.md`); we
/// reject earlier so we never buffer something the service would
/// refuse anyway.
pub const MAX_XISF_BYTES: usize = 128 * 1024 * 1024;

/// Sentinel value written to `photos.platesolve_error` while a solve
/// is in flight. The UI distinguishes "solving" from "failed" by
/// matching on this exact string. Cleared by [`save_result`] /
/// [`save_error`] on completion, or by [`mark_aborted_if_solving`] if
/// the background task panics / the process dies mid-solve.
pub const SOLVING_SENTINEL: &str = "solving";

/// Replaces [`SOLVING_SENTINEL`] when the background task drops without
/// ever calling [`save_result`] or [`save_error`] — i.e. it panicked
/// or the tokio runtime is shutting down. Surfaces to the UI as
/// "you can retry to clear" rather than leaving the photo stuck.
pub const ABORTED_SENTINEL: &str = "stuck: solver aborted, retry to clear";

/// Typed wrapper around the `/v1/solve` HTTP API. Construct once at
/// boot via [`PlatesolveClient::from_config`] and share across the
/// app via `Arc` — `reqwest::Client` is itself an `Arc` of a pool
/// so cloning is cheap.
#[derive(Debug, Clone)]
pub struct PlatesolveClient {
    inner: Client,
    base_url: String,
    api_key: String,
}

impl PlatesolveClient {
    /// Build a client from the populated `[platesolve_*]` config
    /// fields. Returns `Ok(None)` when the base URL is unset (the
    /// feature is opt-in); returns `Err` when the URL is set but the
    /// key is missing or empty (misconfiguration we want to surface
    /// at boot, not at first solve attempt).
    pub fn from_config(config: &Config) -> Result<Option<Self>, AppError> {
        let Some(base_url) = config.platesolve_base_url.as_deref() else {
            return Ok(None);
        };
        let key = config
            .platesolve_api_key
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        if key.is_empty() {
            return Err(AppError::Internal(
                "APP_PLATESOLVE_BASE_URL is set but APP_PLATESOLVE_API_KEY is missing — \
                 the plate-solve service requires bearer auth"
                    .into(),
            ));
        }
        let inner = Client::builder()
            .timeout(Duration::from_secs(config.platesolve_timeout_secs))
            // Disable redirects — the service is HTTPS-direct, any
            // redirect is suspicious (DNS hijack, MITM, captive
            // portal). Fail fast instead.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AppError::Internal(format!("build reqwest client: {e}")))?;
        Ok(Some(Self {
            inner,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: key,
        }))
    }

    /// POST the XISF bytes to `/v1/solve` and return the parsed
    /// response. The optional `caller_options` shadow any header-
    /// derived hint (caller wins per field on the server side).
    pub async fn solve_xisf(
        &self,
        xisf_bytes: Bytes,
        filename: &str,
        caller_options: Option<&SolveOptions>,
    ) -> Result<PlatesolveResult, PlatesolveError> {
        let part = multipart::Part::stream(reqwest::Body::from(xisf_bytes))
            .file_name(filename.to_string())
            .mime_str("application/x-xisf")
            .map_err(|e| PlatesolveError::Internal(format!("mime construct: {e}")))?;
        let mut form = multipart::Form::new().part("image", part);
        if let Some(opts) = caller_options {
            let json = serde_json::to_string(opts)
                .map_err(|e| PlatesolveError::Internal(format!("options serialize: {e}")))?;
            form = form.text("options", json);
        }
        let url = format!("{}/v1/solve", self.base_url);
        let resp = self
            .inner
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(PlatesolveError::from_reqwest)?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<PlatesolveResult>()
                .await
                .map_err(|e| PlatesolveError::Internal(format!("decode success body: {e}")));
        }
        // Try to parse the structured error body emitted by the
        // server's `AppError::IntoResponse`. If decoding fails fall
        // back to the raw status — happens for upstream-proxy errors
        // (Caddy 502, body-limit 413 at the proxy) which return
        // HTML / plain text.
        let body_bytes = resp.bytes().await.unwrap_or_default();
        let parsed: Option<ServerErrorBody> = serde_json::from_slice(&body_bytes).ok();
        Err(PlatesolveError::from_status(status, parsed))
    }
}

// ─────────────────────────────────────────────────────── request types

/// Optional hint overrides for the solve. Every field set takes
/// precedence over what the server extracts from the XISF header.
/// Mirrors `xisf-rs-platesolve-server::handlers::solve::SolveOptions`
/// — keep field names in sync.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct SolveOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ra_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dec_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_scale_arcsec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flip_x: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obs_epoch_jyear: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_catalog_stars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude_limit: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detection_threshold: Option<f64>,
    /// Ask the service to bundle a display-ready JPEG render of the
    /// XISF in the response. When `Some(true)`, the response gains a
    /// [`Render`] field. Default `None` (no render) keeps the
    /// existing contract for callers that only want the WCS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render: Option<bool>,
}

// ─────────────────────────────────────────────────────── response types

/// Successful solve. Field names mirror the server's `SolveResp`.
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct PlatesolveResult {
    pub wcs: Wcs,
    pub rms_arcsec: f64,
    pub matched_count: usize,
    pub detected_count: usize,
    pub iterations: u32,
    pub obs_epoch_jyear: f64,
    pub hint_source: HintSource,
    pub fits: Vec<FitsKeyword>,
    pub pcl_properties: Vec<PclProperty>,
    pub has_distortion: bool,
    pub elapsed_ms: u64,
    /// Present iff `SolveOptions::render == Some(true)` AND the
    /// service successfully decoded the XISF into a JPEG. Render
    /// failure on the success path is non-fatal — the field is
    /// omitted and the WCS still ships, so callers must `match`
    /// rather than assume.
    #[serde(default)]
    pub render: Option<Render>,
}

/// Display-ready JPEG bundled with the solve response. The bytes are
/// base64-encoded over the wire (33 % size inflation accepted in
/// exchange for keeping the multipart parser simple on both ends).
/// Decode with `base64::engine::general_purpose::STANDARD.decode(...)`
/// before persisting to S3.
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Render {
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub bytes_b64: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Wcs {
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub pixel_scale_arcsec: f64,
    pub rotation_deg: f64,
    pub flip_x: bool,
    pub crpix_x: f64,
    pub crpix_y: f64,
    pub cd: [[f64; 2]; 2],
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FitsKeyword {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct PclProperty {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct HintSource {
    pub ra: String,
    pub dec: String,
    pub scale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
}

// ─────────────────────────────────────────────────────── error model

/// Structured failure modes returned by `/v1/solve`. Each variant
/// maps to a documented status code per the service spec; callers
/// can branch on the variant to decide retry / surface-to-user /
/// give-up behaviour.
#[derive(Debug, Error)]
pub enum PlatesolveError {
    /// Caller didn't provide a hint and the XISF header had none
    /// either. The UI should ask the user to fill RA/Dec/scale.
    /// HTTP 422 with `error="no-hint-available"`.
    #[error("plate-solve needs a hint — pass RA/Dec/scale or upload an XISF with header metadata")]
    NoHintAvailable,
    /// The solver ran but didn't converge (too few matches, wrong
    /// hint, sparse field, bad scale). Not retryable with the same
    /// inputs. HTTP 422.
    #[error("solve failed: {0}")]
    SolveFailed(String),
    /// The XISF parser rejected the upload (bad signature, malformed
    /// header, over-limit dimensions). HTTP 415.
    #[error("unsupported XISF: {0}")]
    UnsupportedMedia(String),
    /// 400 — malformed multipart, bad options JSON, etc.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// 401 — API key wrong. Operator problem, not user-facing.
    #[error("plate-solve service rejected our API key")]
    Unauthorized,
    /// 413 — upload too large for the configured server limit
    /// (256 MB by default).
    #[error("XISF too large for the plate-solve service (max 256 MB)")]
    PayloadTooLarge,
    /// 429 — service rate-limited us (per-key or per-IP). Retry
    /// after `retry_after_secs`.
    #[error("plate-solve service rate-limited; retry in {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },
    /// 503 — service queue full or hit the wall-clock cap. Retry
    /// after `retry_after_secs`.
    #[error("plate-solve service unavailable; retry in {retry_after_secs}s")]
    ServiceUnavailable { retry_after_secs: u32 },
    /// Network / TLS / DNS / timeout — anything before the server
    /// returned a status code.
    #[error("network failure talking to plate-solve service: {0}")]
    Transport(String),
    /// 5xx from the server, or a body shape we couldn't decode.
    /// Operator should check the service logs.
    #[error("plate-solve internal error: {0}")]
    Internal(String),
}

impl PlatesolveError {
    fn from_status(status: StatusCode, body: Option<ServerErrorBody>) -> Self {
        let detail = body
            .as_ref()
            .map(|b| b.detail.clone())
            .unwrap_or_else(|| format!("status {status}"));
        let retry_after = body.as_ref().and_then(|b| b.retry_after_secs).unwrap_or(30);
        match status {
            StatusCode::UNPROCESSABLE_ENTITY => match body.as_ref().map(|b| b.error.as_str()) {
                Some("no-hint-available") => Self::NoHintAvailable,
                _ => Self::SolveFailed(detail),
            },
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Self::UnsupportedMedia(detail),
            StatusCode::BAD_REQUEST => Self::BadRequest(detail),
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited {
                retry_after_secs: retry_after,
            },
            StatusCode::SERVICE_UNAVAILABLE => Self::ServiceUnavailable {
                retry_after_secs: retry_after,
            },
            _ => Self::Internal(format!("unexpected status {status}: {detail}")),
        }
    }

    fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Transport(format!("timeout: {e}"))
        } else if e.is_connect() {
            Self::Transport(format!("connection failed: {e}"))
        } else {
            Self::Transport(format!("{e}"))
        }
    }
}

/// Internal representation of the server's
/// `{error, detail, retry_after_secs?, required?}` JSON body.
#[derive(Debug, Deserialize)]
struct ServerErrorBody {
    error: String,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    retry_after_secs: Option<u32>,
}

// ─────────────────────────────────────────────────────── persistence

/// Write the successful solve onto the photo's database row. RA/Dec
/// overwrite the existing columns (which were previously manually
/// surfaced via the verify form); the ancillary telemetry lands in
/// the `platesolve_*` columns added by migration 0021.
///
/// On a previous failed attempt (`platesolve_error` was set), this
/// clears the error so the success replaces the failure cleanly.
pub async fn save_result(
    pool: &PgPool,
    photo_id: Uuid,
    result: &PlatesolveResult,
) -> Result<(), AppError> {
    let embed_json = serde_json::json!({
        "wcs": result.wcs,
        "fits": result.fits,
        "pcl_properties": result.pcl_properties,
        "hint_source": result.hint_source,
        "rms_arcsec": result.rms_arcsec,
        "matched_count": result.matched_count,
        "detected_count": result.detected_count,
        "iterations": result.iterations,
        "obs_epoch_jyear": result.obs_epoch_jyear,
        "has_distortion": result.has_distortion,
    });
    let solved_at = Utc::now();

    // Open a transaction so the solve writes + celestial identification land
    // atomically. Identification runs after the photos UPDATE and queries the
    // freshly-written row; if it fails, the solve is rolled back too (the
    // user gets to retry from a clean state instead of an inconsistent one).
    let mut tx = pool.begin().await?;

    // Measured FRAMING from the solve: the plate-solve pixel scale is the
    // ground truth of the optical train (it captures the actual reducer /
    // spacing the night was shot at), so it beats any catalog/theoretical
    // focal. focal_mm = 206.265 × effective_pixel_µm ÷ scale_arcsec, where
    // effective pixel = sensor pixel × binning. Pixel size comes from the
    // XISF header (XPIXSZ, most reliable) first, the camera catalog second;
    // aperture diameter from the telescope catalog gives the f-ratio.
    // Computed/stored unconditionally on solve — the solve overwrites a
    // theoretical value an applied setup may have written first.
    let fits_f64 = |name: &str| -> Option<f64> {
        result
            .fits
            .iter()
            .find(|k| k.name.eq_ignore_ascii_case(name))
            .and_then(|k| k.value.trim().trim_matches('\'').trim().parse::<f64>().ok())
    };
    let scale = result.wcs.pixel_scale_arcsec;
    let mut pixel_um = fits_f64("XPIXSZ");
    if pixel_um.is_none() {
        // Catalog fallback: photos.camera (freetext) → camera_specs. The
        // camera column matches a catalog display_name exactly when it was
        // set from a setup; a hand-typed mismatch simply yields no fallback.
        pixel_um = sqlx::query_scalar::<_, Option<f64>>(
            r#"select cs.pixel_size_um::float8
                 from photos p
                 join equipment_items ei on ei.kind = 'camera' and ei.display_name = p.camera
                 join camera_specs cs on cs.item_id = ei.id
                where p.id = $1"#,
        )
        .bind(photo_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
    }
    let binning = fits_f64("XBINNING").filter(|b| *b > 0.0).unwrap_or(1.0);
    let derived_focal_mm: Option<f64> = match (pixel_um, scale) {
        (Some(px), s) if px > 0.0 && s > 0.0 => {
            Some((206.265 * px * binning / s * 10.0).round() / 10.0)
        }
        _ => None,
    };
    let derived_aperture_f: Option<f32> = match derived_focal_mm {
        Some(focal) => {
            let aperture_mm = sqlx::query_scalar::<_, Option<i32>>(
                r#"select ts.aperture_mm
                     from photos p
                     join equipment_items ei on ei.kind = 'telescope' and ei.display_name = p.scope
                     join telescope_specs ts on ts.item_id = ei.id
                    where p.id = $1"#,
            )
            .bind(photo_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
            aperture_mm
                .filter(|a| *a > 0)
                .map(|a| ((focal / f64::from(a) * 100.0).round() / 100.0) as f32)
        }
        None => None,
    };

    // Solver-frame dimensions, persisted BEFORE `celestial::identify` runs
    // below: identify reads photos.width/height to project the FOV and
    // silently no-ops when they are NULL (the case for a primary XISF
    // upload, where the JPEG pipeline never ran) — and a stale
    // thumbnail-sized value (JPEG-pipeline photos) shrinks the search
    // radius several-fold. The bundled render is the frame the solver ran
    // on (see the persist_render comment in platesolve_upload.rs), so its
    // dimensions match `pixel_scale_arcsec`; FITS NAXIS1/NAXIS2 are the
    // fallback when the render is absent. persist_render later writes the
    // same values again, which is idempotent.
    let solve_width: Option<i32> = result
        .render
        .as_ref()
        .map(|r| r.width as i32)
        .or_else(|| fits_f64("NAXIS1").map(|v| v as i32))
        .filter(|v| *v > 0);
    let solve_height: Option<i32> = result
        .render
        .as_ref()
        .map(|r| r.height as i32)
        .or_else(|| fits_f64("NAXIS2").map(|v| v as i32))
        .filter(|v| *v > 0);

    // Runtime query (not `sqlx::query!`) on purpose — the new
    // `platesolve_*` columns ship with migration 0021. Until
    // `cargo sqlx prepare` is rerun against a DB that has the
    // migration applied, the compile-time-checked macro would fail
    // the build. The runtime form trades off-the-shelf type checking
    // for the ability to ship the module and the migration together.
    // Promote to `sqlx::query!` after regenerating .sqlx/.
    sqlx::query(
        r#"
        update photos
           set ra_deg                        = $1,
               dec_deg                       = $2,
               platesolve_pixel_scale_arcsec = $3,
               platesolve_rotation_deg       = $4,
               platesolve_rms_arcsec         = $5,
               platesolve_matched_count      = $6,
               platesolve_detected_count     = $7,
               platesolve_solved_at          = $8,
               platesolve_embed_json         = $9,
               -- Measured framing overrides any theoretical value; only
               -- written when derived (pixel size + scale known).
               focal_mm   = coalesce($11, focal_mm),
               aperture_f = coalesce($12, aperture_f),
               -- Solver-frame dims (see above): overwrite stale values
               -- when known, keep the existing ones otherwise.
               width      = coalesce($13, width),
               height     = coalesce($14, height),
               platesolve_error              = null
         where id = $10
        "#,
    )
    .bind(result.wcs.ra_deg)
    .bind(result.wcs.dec_deg)
    .bind(result.wcs.pixel_scale_arcsec as f32)
    .bind(result.wcs.rotation_deg as f32)
    .bind(result.rms_arcsec as f32)
    .bind(result.matched_count as i32)
    .bind(result.detected_count as i32)
    .bind(solved_at)
    .bind(embed_json)
    .bind(photo_id)
    .bind(derived_focal_mm)
    .bind(derived_aperture_f)
    .bind(solve_width)
    .bind(solve_height)
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?;

    // Identify catalogued objects in the now-known FOV and write
    // photo_targets rows with source='plate_solve'. Failure here aborts
    // the whole tx — see the comment above pool.begin().
    let identify_outcome = crate::celestial::identify(photo_id, &mut tx).await?;
    tx.commit().await?;

    info!(
        photo_id = %photo_id,
        rms_arcsec = result.rms_arcsec,
        matched = result.matched_count,
        focal_mm = ?derived_focal_mm,
        aperture_f = ?derived_aperture_f,
        celestial_found = identify_outcome.found,
        celestial_kept = identify_outcome.kept,
        "plate-solve persisted"
    );
    Ok(())
}

/// Record a failed solve attempt. The success path will clear the
/// `platesolve_error` column on the next successful run. Surfaces a
/// short human-readable reason — full diagnostic detail goes to
/// `tracing`.
pub async fn save_error(
    pool: &PgPool,
    photo_id: Uuid,
    error: &PlatesolveError,
) -> Result<(), AppError> {
    let reason = error.to_string();
    warn!(
        photo_id = %photo_id,
        error = %error,
        "plate-solve failure recorded"
    );
    // Runtime query — see note in `save_result` above.
    sqlx::query(r#"update photos set platesolve_error = $1 where id = $2"#)
        .bind(&reason)
        .bind(photo_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
    Ok(())
}

/// Outcome of an atomic attempt to mark a photo as "solving in
/// progress". The handler maps each variant to a distinct status
/// code (404 / 409 / 202).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// Sentinel was set; caller now owns the lock and must call
    /// either `save_result` / `save_error` (success path) or
    /// [`mark_aborted_if_solving`] (drop-guard path) before the
    /// background task ends.
    Claimed,
    /// Photo doesn't exist or isn't owned by `user_id`.
    NotFound,
    /// `platesolve_error` was already set to [`SOLVING_SENTINEL`].
    /// Another solve is in flight; bounce the caller with 409.
    AlreadySolving,
}

/// Atomically mark a photo as "solving" iff it's owned by `user_id`
/// and not already mid-solve. One UPDATE under the hood — no TOCTOU
/// between a separate ownership check and the sentinel write.
///
/// Note that *retrying* a previously-failed solve is allowed: any
/// non-null `platesolve_error` that isn't the in-flight sentinel
/// gets overwritten. The success path (`save_result`) clears the
/// sentinel cleanly when the new attempt converges.
pub async fn try_claim(
    pool: &PgPool,
    photo_id: Uuid,
    user_id: Uuid,
) -> Result<ClaimOutcome, AppError> {
    // Two queries: a cheap ownership check to distinguish 404 from
    // 409, then the atomic claim. The race window between them is
    // benign — a concurrent delete makes the claim return 0 rows,
    // which we report as AlreadySolving and the caller sees 409;
    // a concurrent solve attempt simply means one wins the claim
    // and the other is correctly rejected.
    let owner: Option<Uuid> = sqlx::query_scalar(r#"select owner_id from photos where id = $1"#)
        .bind(photo_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?;
    let Some(owner_id) = owner else {
        return Ok(ClaimOutcome::NotFound);
    };
    if owner_id != user_id {
        // Same as upload_finalize: hide existence under 404 rather
        // than reveal it via 403.
        return Ok(ClaimOutcome::NotFound);
    }
    // Runtime query — see note in `save_result`.
    let rows = sqlx::query(
        r#"
        update photos
           set platesolve_error = $1
         where id = $2
           and (platesolve_error is null or platesolve_error <> $1)
        "#,
    )
    .bind(SOLVING_SENTINEL)
    .bind(photo_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?
    .rows_affected();
    if rows == 0 {
        Ok(ClaimOutcome::AlreadySolving)
    } else {
        Ok(ClaimOutcome::Claimed)
    }
}

/// Drop-guard cleanup: if the spawned background task is dropped
/// (panic, runtime shutdown) while the sentinel is still set, swap
/// the sentinel for [`ABORTED_SENTINEL`] so the photo is resolvable
/// again on the next user action. Best-effort; logs but does not
/// propagate errors (we're already in a failure path).
///
/// Fires-and-forgets a tokio task because `Drop` is synchronous. If
/// the runtime is itself shutting down the spawn won't run — this is
/// the documented gap. Future work: a periodic `photos::cleanup`
/// sweep that ages out `platesolve_error = SOLVING_SENTINEL` rows
/// older than N minutes.
pub fn mark_aborted_if_solving(pool: PgPool, photo_id: Uuid) {
    tokio::spawn(async move {
        // Runtime query — see note in `save_result`.
        let res = sqlx::query(
            r#"update photos set platesolve_error = $1 where id = $2 and platesolve_error = $3"#,
        )
        .bind(ABORTED_SENTINEL)
        .bind(photo_id)
        .bind(SOLVING_SENTINEL)
        .execute(&pool)
        .await;
        if let Err(e) = res {
            // Don't bubble — we're already a cleanup path. Visibility
            // via tracing is enough.
            warn!(
                photo_id = %photo_id,
                error = %e,
                "failed to mark aborted plate-solve sentinel; operator may need to clear manually"
            );
        }
    });
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn config_with_base_url(base: Option<&str>, key: Option<&str>) -> Config {
        Config {
            bind: "0.0.0.0:0".into(),
            log: "info".into(),
            database_url: "postgres://x".into(),
            session_secure: false,
            public_base_url: "http://localhost".into(),
            s3_endpoint: None,
            s3_region: "us-east-1".into(),
            s3_bucket: "b".into(),
            s3_access_key: "a".into(),
            s3_secret_key: "s".into(),
            s3_path_style: true,
            cdn_base_url: "http://localhost".into(),
            cdn_local_fallback: false,
            cors_origin: None,
            oauth_google_client_id: String::new(),
            oauth_google_client_secret: String::new(),
            oauth_google_redirect_url: String::new(),
            smtp_host: "localhost".into(),
            smtp_port: 1025,
            smtp_user: String::new(),
            smtp_pass: String::new(),
            mail_from: "x@x".into(),
            smtp_tls: false,
            platesolve_base_url: base.map(str::to_string),
            platesolve_api_key: key.map(str::to_string),
            platesolve_timeout_secs: 90,
        }
    }

    #[test]
    fn from_config_returns_none_when_base_url_unset() {
        let cfg = config_with_base_url(None, None);
        let result = PlatesolveClient::from_config(&cfg).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn from_config_errors_when_key_missing() {
        let cfg = config_with_base_url(Some("https://platesolve.astrophoto.pics"), None);
        let err = PlatesolveClient::from_config(&cfg).expect_err("must fail without key");
        match err {
            AppError::Internal(msg) => assert!(msg.contains("APP_PLATESOLVE_API_KEY")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn from_config_builds_client() {
        let cfg = config_with_base_url(
            Some("https://platesolve.astrophoto.pics/"),
            Some("test-key"),
        );
        let client = PlatesolveClient::from_config(&cfg)
            .expect("ok")
            .expect("client built");
        // Trailing slash gets stripped so URLs concatenate cleanly.
        assert_eq!(client.base_url, "https://platesolve.astrophoto.pics");
        assert_eq!(client.api_key, "test-key");
    }

    #[test]
    fn from_status_maps_no_hint_to_typed_variant() {
        let body = ServerErrorBody {
            error: "no-hint-available".into(),
            detail: "no hint".into(),
            retry_after_secs: None,
        };
        let e = PlatesolveError::from_status(StatusCode::UNPROCESSABLE_ENTITY, Some(body));
        assert!(matches!(e, PlatesolveError::NoHintAvailable));
    }

    #[test]
    fn from_status_maps_solve_failed() {
        let body = ServerErrorBody {
            error: "solve-failed".into(),
            detail: "too few stars".into(),
            retry_after_secs: None,
        };
        let e = PlatesolveError::from_status(StatusCode::UNPROCESSABLE_ENTITY, Some(body));
        match e {
            PlatesolveError::SolveFailed(s) => assert_eq!(s, "too few stars"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn from_status_preserves_retry_after_on_rate_limit() {
        let body = ServerErrorBody {
            error: "rate-limited".into(),
            detail: "slow down".into(),
            retry_after_secs: Some(45),
        };
        let e = PlatesolveError::from_status(StatusCode::TOO_MANY_REQUESTS, Some(body));
        match e {
            PlatesolveError::RateLimited { retry_after_secs } => assert_eq!(retry_after_secs, 45),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn from_status_falls_back_to_internal_on_unknown_status() {
        let e = PlatesolveError::from_status(StatusCode::IM_A_TEAPOT, None);
        assert!(matches!(e, PlatesolveError::Internal(_)));
    }

    #[test]
    fn parses_response_with_render_field() {
        // New shape: the server included `render` because the caller
        // passed `options.render = true`. The deserializer must pick
        // it up cleanly.
        let body = r#"{
            "wcs": { "ra_deg": 1, "dec_deg": 2, "pixel_scale_arcsec": 1.0,
                     "rotation_deg": 0, "flip_x": false,
                     "crpix_x": 0, "crpix_y": 0, "cd": [[1,0],[0,1]] },
            "rms_arcsec": 1.0, "matched_count": 10, "detected_count": 100,
            "iterations": 1, "obs_epoch_jyear": 2024.5,
            "hint_source": { "ra": "x", "dec": "x", "scale": "x" },
            "fits": [], "pcl_properties": [],
            "has_distortion": false, "elapsed_ms": 100,
            "render": { "mime": "image/jpeg", "width": 100, "height": 100,
                        "bytes_b64": "AAAA" }
        }"#;
        let r: PlatesolveResult = serde_json::from_str(body).expect("parse");
        let render = r.render.expect("render present");
        assert_eq!(render.mime, "image/jpeg");
        assert_eq!(render.width, 100);
        assert_eq!(render.height, 100);
        assert_eq!(render.bytes_b64, "AAAA");
    }

    #[test]
    fn parses_response_without_render_field() {
        // Back-compat: callers that don't pass `options.render = true`
        // (or services older than the render rollout) return a body
        // with NO `render` key. `#[serde(default)]` on the field
        // means deserialization must still succeed with `None`.
        let body = r#"{
            "wcs": { "ra_deg": 1, "dec_deg": 2, "pixel_scale_arcsec": 1.0,
                     "rotation_deg": 0, "flip_x": false,
                     "crpix_x": 0, "crpix_y": 0, "cd": [[1,0],[0,1]] },
            "rms_arcsec": 1.0, "matched_count": 10, "detected_count": 100,
            "iterations": 1, "obs_epoch_jyear": 2024.5,
            "hint_source": { "ra": "x", "dec": "x", "scale": "x" },
            "fits": [], "pcl_properties": [],
            "has_distortion": false, "elapsed_ms": 100
        }"#;
        let r: PlatesolveResult = serde_json::from_str(body).expect("parse");
        assert!(r.render.is_none());
    }
}
