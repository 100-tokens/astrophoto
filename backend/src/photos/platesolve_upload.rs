//! `POST /api/photos/:id/platesolve` — side-channel XISF upload that
//! triggers a plate-solve via the external service.
//!
//! The XISF is **not stored**. We buffer it in memory, forward it to
//! `xisf-rs-platesolve-server`, persist the WCS result onto the
//! photo's row, and discard the bytes. See
//! `docs/platesolve-integration.md` for the design discussion.
//!
//! Flow:
//! 1. Owner-checked, atomically claim the in-progress sentinel
//!    ([`platesolve::try_claim`]). 404/409/proceed.
//! 2. Acquire a semaphore permit so we cap concurrent solves (bounds
//!    RSS on the Koyeb tier).
//! 3. Magic-byte sniff the body (must be XISF).
//! 4. Parse the optional `options` JSON multipart field.
//! 5. Spawn a background task that calls the plate-solve client with
//!    bounded retries on transient failures; the task owns a drop
//!    guard that swaps the sentinel for [`ABORTED_SENTINEL`] if it
//!    panics or the runtime shuts down.
//! 6. Return 202 — the result lands in the DB asynchronously.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::magic::{self, SniffResult};
use crate::photos::platesolve::{
    self, ClaimOutcome, MAX_XISF_BYTES, PlatesolveClient, PlatesolveError, Render, SolveOptions,
};
use crate::storage::Storage;

/// Maximum number of attempts (including the initial one) before
/// we give up on retryable failures (rate-limit / 503 / transport).
const MAX_ATTEMPTS: u32 = 3;

/// Backoff cap so a server-provided `retry_after_secs` can't pin a
/// worker for an unbounded amount of time on a misbehaving service.
const BACKOFF_CAP: Duration = Duration::from_secs(60);

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    // Route is conditionally mounted on `state.platesolve.is_some()`,
    // so this should never trigger in practice — kept as a defensive
    // 500 for the test paths that build a router without the client.
    let client = state
        .platesolve
        .as_ref()
        .ok_or_else(|| AppError::Internal("plate-solve service not configured".into()))?
        .clone();

    // Parse the multipart up front (cheap; body is already in memory
    // thanks to the route's DefaultBodyLimit). We need the bytes
    // before claiming the sentinel so a malformed payload doesn't
    // leave a stale 'solving' row to clean up.
    let parts = parse_multipart(multipart, id).await?;
    let sig = magic::sniff(&parts.xisf_bytes);
    if sig != SniffResult::Xisf {
        return Err(AppError::MagicByteMismatch(format!(
            "expected XISF signature, sniffed {sig:?}"
        )));
    }

    // Atomic claim — owner check + sentinel write in one place. Maps
    // to 404 (not yours / doesn't exist) or 409 (already solving).
    match platesolve::try_claim(&state.pool, id, user.id).await? {
        ClaimOutcome::Claimed => {}
        ClaimOutcome::NotFound => return Err(AppError::not_found("photo")),
        ClaimOutcome::AlreadySolving => {
            return Err(AppError::Conflict("plate-solve already in progress".into()));
        }
    }

    // Spawn the background solve. The handler returns 202 immediately;
    // the result lands on the photo row asynchronously. The side-channel
    // flow enriches an already-`ready` photo with WCS + display JPEG and
    // does NOT transition `photos.status` — the row stays whatever it
    // was (typically `ready`). The auto-trigger flow on a primary XISF
    // upload reads the same `run_solve` result and does transition status.
    let pool = state.pool.clone();
    let storage = Arc::clone(&state.storage);
    let permits = Arc::clone(&state.platesolve_permits);
    tokio::spawn(async move {
        let _ = run_solve(
            pool,
            storage,
            permits,
            client,
            id,
            parts.xisf_bytes,
            parts.filename,
            parts.options,
        )
        .await;
    });

    Ok(StatusCode::ACCEPTED)
}

struct UploadParts {
    xisf_bytes: Bytes,
    filename: String,
    options: Option<SolveOptions>,
}

async fn parse_multipart(mut mp: Multipart, photo_id: Uuid) -> Result<UploadParts, AppError> {
    let mut xisf_bytes: Option<Bytes> = None;
    let mut filename = format!("{photo_id}.xisf");
    let mut options: Option<SolveOptions> = None;
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Validation(format!("read file part: {e}")))?;
                if data.len() > MAX_XISF_BYTES {
                    return Err(AppError::PayloadTooLarge(format!(
                        "XISF body is {} bytes, max {MAX_XISF_BYTES}",
                        data.len()
                    )));
                }
                xisf_bytes = Some(data);
            }
            Some("options") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::Validation(format!("read options part: {e}")))?;
                let parsed: SolveOptions = serde_json::from_str(&text)
                    .map_err(|e| AppError::Validation(format!("options JSON: {e}")))?;
                options = Some(parsed);
            }
            // Unknown fields are ignored so the multipart shape can
            // grow without breaking older clients.
            _ => {}
        }
    }
    let xisf_bytes =
        xisf_bytes.ok_or_else(|| AppError::Validation("missing `file` part".into()))?;
    Ok(UploadParts {
        xisf_bytes,
        filename,
        options,
    })
}

/// Run a plate-solve for `photo_id` against the upstream service,
/// persist the WCS + bundled JPEG, and report the outcome.
///
/// Shared by the side-channel `POST /api/photos/:id/platesolve`
/// handler (which discards the return value — the photo row stays
/// whatever status it was) and the primary-XISF-upload auto-trigger
/// (which uses the return value to transition `photos.status` to
/// `ready` or `failed`).
///
/// The function owns:
/// - the semaphore permit (acquired internally so the queue depth
///   bounds RSS),
/// - the sentinel drop-guard (fires on panic / runtime shutdown so
///   the photo never stays stuck in `platesolve_error='solving'`).
///
/// Callers MUST have already claimed the sentinel via
/// [`platesolve::try_claim`] — `run_solve` does not re-claim.
#[allow(clippy::too_many_arguments)]
pub async fn run_solve(
    pool: PgPool,
    storage: Arc<dyn Storage>,
    permits: Arc<Semaphore>,
    client: Arc<PlatesolveClient>,
    photo_id: Uuid,
    xisf_bytes: Bytes,
    filename: String,
    options: Option<SolveOptions>,
) -> Result<platesolve::PlatesolveResult, PlatesolveError> {
    // Acquire the concurrency permit. `acquire_owned` so the permit
    // lives inside the spawned task without borrowing.
    let permit = match Arc::clone(&permits).acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            // Semaphore was closed — only happens during shutdown.
            // Let the drop guard mark the sentinel aborted.
            warn!(
                photo_id = %photo_id,
                "plate-solve permit semaphore closed before acquire"
            );
            platesolve::mark_aborted_if_solving(pool, photo_id);
            return Err(PlatesolveError::Internal("permit semaphore closed".into()));
        }
    };

    // Drop guard armed for the duration of the solve. Disarmed below
    // once `save_result` / `save_error` has cleared the sentinel.
    let guard = SentinelGuard::new(pool.clone(), photo_id);

    // Force render=true: both consumers (side-channel + auto-trigger)
    // want the display JPEG. User-supplied options are preserved for
    // everything else.
    let mut effective_options = options.unwrap_or_default();
    effective_options.render = Some(true);

    let outcome = solve_with_retries(
        &client,
        photo_id,
        &xisf_bytes,
        &filename,
        Some(&effective_options),
    )
    .await;
    let result = match outcome {
        Ok(result) => {
            if let Err(e) = platesolve::save_result(&pool, photo_id, &result).await {
                // DB write failed after a successful solve. Record the
                // DB error so the row doesn't stay 'solving' forever;
                // operator sees this in tracing.
                warn!(photo_id=%photo_id, error=%e, "save_result failed after successful solve");
                let mapped = PlatesolveError::Internal(format!("DB write after solve: {e}"));
                let _ = platesolve::save_error(&pool, photo_id, &mapped).await;
                Err(mapped)
            } else if let Some(render) = &result.render {
                // Persist the bundled JPEG as the display master so
                // the photo renders in the gallery. Render failure is
                // logged but doesn't undo the successful solve — the
                // WCS row is the load-bearing artifact, display can
                // be re-derived later.
                if let Err(e) = persist_render(&storage, &pool, photo_id, render).await {
                    warn!(
                        photo_id = %photo_id,
                        error = %e,
                        "platesolve render persist failed; display_key not updated"
                    );
                }
                Ok(result)
            } else {
                warn!(
                    photo_id = %photo_id,
                    "platesolve render missing from response; display_key not updated"
                );
                Ok(result)
            }
        }
        Err(e) => {
            if let Err(db_err) = platesolve::save_error(&pool, photo_id, &e).await {
                warn!(photo_id=%photo_id, error=%db_err, "save_error failed");
            }
            Err(e)
        }
    };

    // Save path completed (success or failure). Disarm the guard so
    // Drop is a no-op.
    guard.disarm();
    drop(permit);
    result
}

/// Auto-trigger entry point for the primary-XISF-upload flow.
///
/// Fires fire-and-forget from `upload_finalize::handler` after a
/// successful XISF finalize transitions the photo to
/// `awaiting-calibration`. Fetches the XISF from S3, claims the
/// sentinel, calls [`run_solve`], and transitions `photos.status`
/// to `ready` (success) or `failed` (terminal solve error).
///
/// Same back-pressure as the side-channel POST: bounded by the
/// `platesolve_permits` semaphore so concurrent uploads queue.
pub fn auto_calibrate_xisf(state: AppState, photo_id: Uuid, storage_key: String, owner_id: Uuid) {
    // Conditionally mounted at boot — if no client is configured,
    // this function is never reached. The caller already checked
    // `state.platesolve.is_some()` and returned UnsupportedFormat
    // at upload_init time.
    let Some(client) = state.platesolve.clone() else {
        warn!(
            photo_id = %photo_id,
            "auto-calibrate spawned without a configured plate-solve client; marking failed"
        );
        let pool = state.pool.clone();
        tokio::spawn(async move {
            let _ = sqlx::query("update photos set status='failed', pipeline_error=$1 where id=$2")
                .bind("plate-solve not configured on this deployment")
                .bind(photo_id)
                .execute(&pool)
                .await;
        });
        return;
    };

    tokio::spawn(async move {
        // Fetch the XISF bytes from S3. The original was uploaded via
        // the presigned PUT and is sitting at `storage_key`. Missing
        // object = the PUT never landed — should never happen because
        // upload_finalize verified the object first via storage.get.
        let xisf_bytes = match state.storage.get(&storage_key).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                warn!(
                    photo_id = %photo_id,
                    storage_key = %storage_key,
                    "auto-calibrate: XISF original missing from S3"
                );
                mark_calibration_failed(
                    &state.pool,
                    photo_id,
                    "XISF original missing from storage",
                )
                .await;
                return;
            }
            Err(e) => {
                warn!(
                    photo_id = %photo_id,
                    error = %e,
                    "auto-calibrate: storage.get failed"
                );
                mark_calibration_failed(&state.pool, photo_id, &format!("storage error: {e}"))
                    .await;
                return;
            }
        };

        // Parse the PixInsight processing history straight from the XISF
        // header and persist it. Independent of the solve outcome — even a
        // failed solve still yields a processing report. `Bytes` clone is
        // an Arc bump; the original moves into `run_solve` below.
        persist_processing_report(&state.pool, photo_id, xisf_bytes.clone()).await;

        // Claim the sentinel so a concurrent side-channel POST from
        // the same owner gets 409.
        match platesolve::try_claim(&state.pool, photo_id, owner_id).await {
            Ok(platesolve::ClaimOutcome::Claimed) => {}
            Ok(platesolve::ClaimOutcome::AlreadySolving) => {
                // Side-channel POST raced us — let it finish. We don't
                // need to do anything; the side-channel flow will set
                // platesolve_solved_at but NOT transition status. We
                // need to come back later to transition status — for
                // v1 we just log; a future tick of `photos::cleanup`
                // can sweep `awaiting-calibration` rows whose
                // platesolve_solved_at is now set.
                info!(
                    photo_id = %photo_id,
                    "auto-calibrate: another solve already in flight; skipping"
                );
                return;
            }
            Ok(platesolve::ClaimOutcome::NotFound) => {
                // Photo was deleted between finalize and this spawn.
                // Nothing to do.
                return;
            }
            Err(e) => {
                warn!(
                    photo_id = %photo_id,
                    error = %e,
                    "auto-calibrate: try_claim failed"
                );
                mark_calibration_failed(&state.pool, photo_id, &format!("claim failed: {e}")).await;
                return;
            }
        }

        let filename = format!("{photo_id}.xisf");
        let render_bytes = xisf_bytes.clone();
        let result = run_solve(
            state.pool.clone(),
            Arc::clone(&state.storage),
            Arc::clone(&state.platesolve_permits),
            client,
            photo_id,
            xisf_bytes,
            filename,
            None,
        )
        .await;

        // Render the display JPEG locally and overwrite display/<id>.jpg.
        // The plate-solve service's bundled render stacks planar RGB
        // channels as grayscale; ours combines them correctly. Runs
        // regardless of solve outcome so the image is always right.
        persist_local_render(state.storage.as_ref(), &state.pool, photo_id, render_bytes).await;

        match result {
            Ok(result) => {
                // Pull the XISF header instrumentation (camera /
                // exposure / focal length / gain / sensor temp /
                // taken_at / target / etc.) out of the FITS + PCL
                // arrays the service returned, and write any fields
                // the user hasn't already set. `xisf_meta::apply`
                // uses COALESCE so existing values are preserved.
                let meta = crate::photos::xisf_meta::extract(&result);
                if let Err(e) = crate::photos::xisf_meta::apply(&state.pool, photo_id, &meta).await
                {
                    warn!(
                        photo_id = %photo_id,
                        error = %e,
                        "auto-calibrate: xisf_meta::apply failed — proceeding to mark ready anyway"
                    );
                }

                // Transition to `ready`. `width` / `height` are read
                // from the persisted render telemetry if we have it;
                // otherwise they stay null and the gallery falls back
                // to layout estimates (acceptable for v1).
                if let Err(e) = mark_xisf_ready(&state.pool, photo_id).await {
                    warn!(
                        photo_id = %photo_id,
                        error = %e,
                        "auto-calibrate: mark_xisf_ready failed"
                    );
                }
            }
            Err(e) => {
                // run_solve already wrote `platesolve_error`. Also
                // mark the photo failed so the upload-finalize state
                // machine reflects the terminal failure.
                mark_calibration_failed(&state.pool, photo_id, &e.to_string()).await;
            }
        }
    });
}

/// Status transition after a successful auto-calibrate: read the
/// display master's dimensions out of the render telemetry (now in
/// `platesolve_embed_json`) so the gallery has w/h, then mark ready.
/// Best-effort — dimensions fall back to 0/0 if the read fails.
async fn mark_xisf_ready(pool: &PgPool, photo_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "update photos set status='ready', pipeline_error=null where id=$1 and status='awaiting-calibration'",
    )
    .bind(photo_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;
    Ok(())
}

/// Status transition for terminal auto-calibrate failures. Records
/// the reason in `pipeline_error` so the UI can surface it the same
/// way it surfaces JPEG-pipeline failures.
async fn mark_calibration_failed(pool: &PgPool, photo_id: Uuid, reason: &str) {
    let res = sqlx::query(
        "update photos set status='failed', pipeline_error=$1 where id=$2 and status='awaiting-calibration'",
    )
    .bind(reason)
    .bind(photo_id)
    .execute(pool)
    .await;
    if let Err(e) = res {
        warn!(
            photo_id = %photo_id,
            error = %e,
            reason = %reason,
            "auto-calibrate: failed to mark photo failed"
        );
    }
}

/// Decode the base64 JPEG bundled in the solve response, write it as
/// the display master in S3, and point `photos.display_key` at it.
///
/// Runtime sqlx query for the same reason as the other plate-solve
/// helpers — the column already exists on `photos`, no migration.
async fn persist_render(
    storage: &Arc<dyn Storage>,
    pool: &PgPool,
    photo_id: Uuid,
    render: &Render,
) -> Result<(), AppError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&render.bytes_b64)
        .map_err(|e| AppError::Internal(format!("decode render bytes: {e}")))?;
    let byte_count = bytes.len();
    let key = format!("display/{photo_id}.jpg");
    storage.put(&key, &render.mime, Bytes::from(bytes)).await?;
    // Persist the render's pixel dimensions alongside display_key. These are
    // the dimensions of the frame the plate-solver actually ran on, so they
    // match `platesolve_pixel_scale_arcsec` (arcsec per pixel of THIS frame)
    // and the WCS centre (crpix ≈ width/2, height/2). The celestial overlay's
    // gnomonic projection (frontend wcs.ts) needs both width/height and the
    // pixel scale to place markers — without these columns it cannot compute
    // the field-of-view and `celestial::identify` short-circuits. Casts to
    // i32 are safe: render dimensions are bounded well under i32::MAX.
    sqlx::query("update photos set display_key = $1, width = $2, height = $3 where id = $4")
        .bind(&key)
        .bind(render.width as i32)
        .bind(render.height as i32)
        .bind(photo_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
    info!(
        photo_id = %photo_id,
        bytes = byte_count,
        width = render.width,
        height = render.height,
        "platesolve render persisted as display master"
    );
    Ok(())
}

/// One attempt per loop iteration. On retryable errors, backs off
/// using the server-supplied `retry_after_secs` when present,
/// capped by [`BACKOFF_CAP`]; on terminal errors, returns early.
async fn solve_with_retries(
    client: &PlatesolveClient,
    photo_id: Uuid,
    xisf_bytes: &Bytes,
    filename: &str,
    options: Option<&SolveOptions>,
) -> Result<platesolve::PlatesolveResult, PlatesolveError> {
    let mut last_err: Option<PlatesolveError> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        // Clone Bytes is cheap (refcount).
        match client
            .solve_xisf(xisf_bytes.clone(), filename, options)
            .await
        {
            Ok(r) => {
                info!(
                    photo_id = %photo_id,
                    attempt,
                    "plate-solve succeeded"
                );
                return Ok(r);
            }
            Err(e) => {
                let backoff = match &e {
                    PlatesolveError::RateLimited { retry_after_secs }
                    | PlatesolveError::ServiceUnavailable { retry_after_secs } => {
                        Some(Duration::from_secs(u64::from(*retry_after_secs)).min(BACKOFF_CAP))
                    }
                    PlatesolveError::Transport(_) => {
                        // Exponential: 1s, 2s, 4s.
                        Some(Duration::from_secs(1u64 << (attempt - 1)).min(BACKOFF_CAP))
                    }
                    // Terminal: 400/401/413/415/422/internal — no point retrying.
                    _ => None,
                };
                warn!(
                    photo_id = %photo_id,
                    attempt,
                    error = %e,
                    retry_in_secs = backoff.map(|d| d.as_secs()).unwrap_or(0),
                    "plate-solve attempt failed"
                );
                last_err = Some(e);
                match backoff {
                    Some(d) if attempt < MAX_ATTEMPTS => tokio::time::sleep(d).await,
                    _ => break,
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        PlatesolveError::Internal("retry loop exited without an outcome".into())
    }))
}

/// RAII cleanup for the in-progress sentinel. Disarmed in the happy
/// path once `save_result` / `save_error` has overwritten the
/// sentinel; fires (swap → ABORTED_SENTINEL) only on panic or
/// premature drop.
struct SentinelGuard {
    pool: PgPool,
    photo_id: Uuid,
    armed: bool,
}

impl SentinelGuard {
    fn new(pool: PgPool, photo_id: Uuid) -> Self {
        Self {
            pool,
            photo_id,
            armed: true,
        }
    }
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for SentinelGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        platesolve::mark_aborted_if_solving(self.pool.clone(), self.photo_id);
    }
}

/// Parse the XISF header and persist the [`ProcessingReport`] to
/// `photos.processing_json`. Best-effort: every failure mode is logged
/// and swallowed so it never blocks calibration. The ~100 KB XML parse
/// runs in `spawn_blocking` because this code path is otherwise fully
/// async (the XISF *decode* happens on the remote solver).
///
/// [`ProcessingReport`]: crate::photos::xisf_processing::ProcessingReport
async fn persist_processing_report(pool: &PgPool, photo_id: Uuid, bytes: Bytes) {
    let parsed =
        tokio::task::spawn_blocking(move || crate::photos::xisf_processing::parse_xisf(&bytes))
            .await;
    let report = match parsed {
        Ok(Ok(Some(r))) => r,
        Ok(Ok(None)) => return, // valid XISF, no processing history
        Ok(Err(e)) => {
            warn!(photo_id = %photo_id, error = %e, "xisf processing parse failed");
            return;
        }
        Err(e) => {
            warn!(photo_id = %photo_id, error = %e, "xisf processing parse panicked");
            return;
        }
    };
    let json = match serde_json::to_value(&report) {
        Ok(v) => v,
        Err(e) => {
            warn!(photo_id = %photo_id, error = %e, "serialize processing report");
            return;
        }
    };
    if let Err(e) = sqlx::query("UPDATE photos SET processing_json = $1 WHERE id = $2")
        .bind(json)
        .bind(photo_id)
        .execute(pool)
        .await
    {
        warn!(photo_id = %photo_id, error = %e, "persist processing_json failed");
    }
}

/// Render the XISF display JPEG locally (combining planar RGB channels
/// correctly) and overwrite `display/<id>.jpg`. Fixes the plate-solve
/// service's stacked-grayscale render. Best-effort: on any failure the
/// existing (service) render is left in place.
async fn persist_local_render(storage: &dyn Storage, pool: &PgPool, photo_id: Uuid, bytes: Bytes) {
    // 2560 px long edge: matches the photo page's largest CDN request
    // (w=2560) so the display master is served full-resolution there, while
    // keeping the render cheap enough for the small prod instance. The
    // original XISF is retained for any higher-res need.
    let rendered = tokio::task::spawn_blocking(move || {
        crate::photos::xisf_render::render_display_jpeg(&bytes, 2560)
    })
    .await;
    let jpeg = match rendered {
        Ok(Ok(Some(j))) => j,
        Ok(Ok(None)) => return, // unsupported format — keep existing render
        Ok(Err(e)) => {
            warn!(photo_id = %photo_id, error = %e, "local xisf render failed");
            return;
        }
        Err(e) => {
            warn!(photo_id = %photo_id, error = %e, "local xisf render panicked");
            return;
        }
    };
    let key = format!("display/{photo_id}.jpg");
    if let Err(e) = storage.put(&key, "image/jpeg", Bytes::from(jpeg)).await {
        warn!(photo_id = %photo_id, error = %e, "store local render failed");
        return;
    }
    if let Err(e) = sqlx::query("update photos set display_key = $1 where id = $2")
        .bind(&key)
        .bind(photo_id)
        .execute(pool)
        .await
    {
        warn!(photo_id = %photo_id, error = %e, "update display_key after local render failed");
    }
}
