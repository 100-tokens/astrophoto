//! Pending-upload reaper + stuck-pipeline sweeps.
//!
//! Photos rows with status='pending' that never received a finalize call
//! become orphans (the user closed the tab, lost network, or our cancel
//! endpoint failed to fire). Without this, the photos table grows with
//! garbage rows and the originals bucket leaks objects.
//!
//! TTL is intentionally generous (24 h) so a slow upload on a poor link
//! never triggers a false positive.
//!
//! The same hourly tick also recovers rows a crash or lost race left in
//! a transient status forever: 'awaiting-calibration' (XISF whose
//! background solve died, or whose side-channel solve won the claim and
//! never transitioned status) and 'processing' (finalize/replace died
//! mid-pipeline). Those rows are marked ready/failed — never deleted —
//! so the user can retry and no asset is destroyed.

use std::sync::Arc;
use std::time::Duration;

use crate::AppError;
use crate::photos::platesolve::{ABORTED_SENTINEL, SOLVING_SENTINEL};
use crate::storage::Storage;

const REAP_INTERVAL: Duration = Duration::from_secs(60 * 60); // 1h

/// Run a single reap pass. Returns the number of rows deleted.
pub async fn reap_once<S: Storage + ?Sized>(
    pool: &sqlx::PgPool,
    storage: &S,
) -> Result<u64, AppError> {
    let rows = sqlx::query!(
        r#"
        select id, storage_key from photos
         where status = 'pending'
           and created_at < now() - interval '24 hours'
         limit 500
        "#
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let ids: Vec<uuid::Uuid> = rows.iter().map(|r| r.id).collect();
    let keys: Vec<String> = rows.into_iter().map(|r| r.storage_key).collect();

    let deleted = sqlx::query!("delete from photos where id = any($1)", &ids)
        .execute(pool)
        .await?
        .rows_affected();

    if let Err(e) = storage.delete_objects(&keys).await {
        tracing::warn!(error = %e, count = keys.len(), "cleanup: S3 batch delete failed");
    }

    tracing::info!(count = deleted, "cleanup: reaped pending uploads");
    Ok(deleted)
}

/// Recover photos stuck in a transient pipeline status after a crash,
/// restart, or lost claim race. Runs on the same hourly ticker as
/// [`reap_once`]. Never deletes anything.
pub async fn sweep_stuck_pipeline(pool: &sqlx::PgPool) -> Result<(), AppError> {
    // 1. Lost-race promotion: a side-channel solve won the claim while
    //    the auto-calibrate flow held 'awaiting-calibration'. The solve
    //    landed (platesolve_solved_at set, display rendered) but nobody
    //    transitioned status — promote to ready.
    let promoted = sqlx::query!(
        "update photos set status='ready', pipeline_error=null
          where status='awaiting-calibration' and platesolve_solved_at is not null"
    )
    .execute(pool)
    .await?
    .rows_affected();

    // 2. Interrupted calibration: no solve landed within 30 minutes of
    //    the calibration request. Mark failed so the UI surfaces a retry.
    //    Swapping a stale 'solving' sentinel for the aborted one is
    //    load-bearing: a re-run of finalize would otherwise hit
    //    ClaimOutcome::AlreadySolving and silently no-op.
    let timed_out = sqlx::query!(
        "update photos
            set status='failed',
                pipeline_error='auto-calibration interrupted — retry by re-running finalize',
                platesolve_error = case when platesolve_error = $1 then $2
                                        else platesolve_error end
          where status='awaiting-calibration'
            and platesolve_solved_at is null
            and coalesce(calibration_requested_at, replaced_at, created_at)
                < now() - interval '30 minutes'",
        SOLVING_SENTINEL,
        ABORTED_SENTINEL,
    )
    .execute(pool)
    .await?
    .rows_affected();

    // 3. Interrupted finalize/replace: 'processing' is held only while
    //    the pipeline task is alive (seconds to minutes). Rows older
    //    than 6 hours mean the process died mid-pipeline; resurface as
    //    a retryable failure instead of leaving the photo wedged.
    let interrupted = sqlx::query!(
        "update photos
            set status='failed',
                pipeline_error='processing interrupted — retry the upload or replace'
          where status='processing'
            and coalesce(replaced_at, created_at) < now() - interval '6 hours'"
    )
    .execute(pool)
    .await?
    .rows_affected();

    if promoted + timed_out + interrupted > 0 {
        tracing::info!(
            promoted,
            timed_out,
            interrupted,
            "cleanup: recovered stuck pipeline rows"
        );
    }
    Ok(())
}

/// Spawn a tokio task that calls `reap_once` every hour. Errors are logged
/// and never propagated — we never want the reaper to crash the binary.
pub fn spawn_periodic(pool: sqlx::PgPool, storage: Arc<dyn Storage>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(REAP_INTERVAL);
        // First tick fires immediately; skip it so boot isn't a sync flush.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = reap_once(&pool, storage.as_ref()).await {
                tracing::error!(error = %e, "cleanup: reap_once errored");
            }
            if let Err(e) = sweep_stuck_pipeline(&pool).await {
                tracing::error!(error = %e, "cleanup: sweep_stuck_pipeline errored");
            }
        }
    });
}
