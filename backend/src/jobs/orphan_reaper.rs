//! Orphan reaper: photos stuck in `status = 'pending'` past STALE_HOURS have
//! their S3 originals deleted (best-effort) and rows hard-deleted.
//! Runs every TICK_SECS in the background; one bad tick is logged and skipped.

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;

use crate::AppError;
use crate::storage::Storage;

const STALE_HOURS: i64 = 2;
const TICK_SECS: u64 = 300;

pub fn spawn(pool: PgPool, storage: Arc<dyn Storage>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(TICK_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep_once(&pool, storage.as_ref()).await {
                tracing::error!(error = ?e, "orphan reaper tick failed");
            }
        }
    });
}

pub async fn sweep_once(pool: &PgPool, storage: &dyn Storage) -> Result<(), AppError> {
    let stale = sqlx::query!(
        r#"
        select id as "id!", storage_key
          from photos
         where status = 'pending'
           and created_at < now() - make_interval(hours => $1::int)
         limit 100
        "#,
        STALE_HOURS as i32
    )
    .fetch_all(pool)
    .await?;

    for row in stale {
        let _ = storage.delete(&row.storage_key).await; // best-effort
        sqlx::query!("delete from photos where id = $1", row.id)
            .execute(pool)
            .await?;
        tracing::info!(photo_id = %row.id, "reaped orphan upload");
    }
    Ok(())
}
