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
    // the result lands on the photo row asynchronously.
    let pool = state.pool.clone();
    let storage = Arc::clone(&state.storage);
    let permits = Arc::clone(&state.platesolve_permits);
    tokio::spawn(async move {
        run_solve(
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

/// Owned by the spawned task. The semaphore permit is acquired
/// outside the drop guard so a queue-wait doesn't leave the sentinel
/// pending without back-pressure. The guard fires iff we drop without
/// `disarm()` being called — i.e. on panic / runtime shutdown.
#[allow(clippy::too_many_arguments)]
async fn run_solve(
    pool: PgPool,
    storage: Arc<dyn Storage>,
    permits: Arc<Semaphore>,
    client: Arc<PlatesolveClient>,
    photo_id: Uuid,
    xisf_bytes: Bytes,
    filename: String,
    options: Option<SolveOptions>,
) {
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
            return;
        }
    };

    // Drop guard armed for the duration of the solve. Disarmed below
    // once `save_result` / `save_error` has cleared the sentinel.
    let guard = SentinelGuard::new(pool.clone(), photo_id);

    // Force render=true: the side-channel calibration flow always
    // wants the display JPEG so a calibrated XISF appears in the
    // gallery without a separate JPEG upload. User-supplied options
    // are preserved for everything else.
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
    match outcome {
        Ok(result) => {
            if let Err(e) = platesolve::save_result(&pool, photo_id, &result).await {
                // DB write failed after a successful solve. Record the
                // DB error so the row doesn't stay 'solving' forever;
                // operator sees this in tracing.
                warn!(photo_id=%photo_id, error=%e, "save_result failed after successful solve");
                let _ = platesolve::save_error(
                    &pool,
                    photo_id,
                    &PlatesolveError::Internal(format!("DB write after solve: {e}")),
                )
                .await;
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
            } else {
                warn!(
                    photo_id = %photo_id,
                    "platesolve render missing from response; display_key not updated"
                );
            }
        }
        Err(e) => {
            if let Err(db_err) = platesolve::save_error(&pool, photo_id, &e).await {
                warn!(photo_id=%photo_id, error=%db_err, "save_error failed");
            }
        }
    }

    // Save path completed (success or failure). Disarm the guard so
    // Drop is a no-op.
    guard.disarm();
    drop(permit);
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
    sqlx::query("update photos set display_key = $1 where id = $2")
        .bind(&key)
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
