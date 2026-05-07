//! Pending-upload reaper.
//!
//! Photos rows with status='pending' that never received a finalize call
//! become orphans (the user closed the tab, lost network, or our cancel
//! endpoint failed to fire). Without this, the photos table grows with
//! garbage rows and the originals bucket leaks objects.
//!
//! TTL is intentionally generous (24 h) so a slow upload on a poor link
//! never triggers a false positive.

use std::sync::Arc;
use std::time::Duration;

use crate::AppError;
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
        }
    });
}
