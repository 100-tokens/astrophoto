//! Hourly worker: hard-delete accounts whose grace period has elapsed.
//! Per-user errors are logged and skipped — one bad account never stalls
//! the whole hourly batch.

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::storage::Storage;

pub fn spawn(pool: PgPool, storage: Arc<dyn Storage>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = purge_once(&pool, storage.as_ref()).await {
                tracing::error!(error = ?e, "purge_deletions cycle failed");
            }
        }
    });
}

pub async fn purge_once(pool: &PgPool, storage: &dyn Storage) -> Result<u64, AppError> {
    let due: Vec<Uuid> = sqlx::query_scalar!(
        "select id from users
          where pending_deletion_at is not null
            and pending_deletion_at < now()"
    )
    .fetch_all(pool)
    .await?;

    let mut deleted = 0u64;
    if !due.is_empty() {
        for user_id in &due {
            match purge_one_user(pool, storage, *user_id).await {
                Ok(()) => deleted += 1,
                Err(e) => tracing::error!(
                    user_id = %user_id, error = ?e,
                    "purge_one_user failed; skipping"
                ),
            }
        }
    }

    // Sweep orphaned pending S3 deletes (replace pipeline never reached 'ready').
    match sweep_pending_deletes(pool, storage).await {
        Ok(swept) if swept > 0 => tracing::info!(swept, "sweep_pending_deletes cleared stale rows"),
        Ok(_) => {}
        Err(e) => tracing::error!(error = ?e, "sweep_pending_deletes failed"),
    }

    tracing::info!(deleted, total_due = due.len(), "purge cycle done");
    Ok(deleted)
}

pub async fn sweep_pending_deletes(
    pool: &PgPool,
    storage: &dyn Storage,
) -> Result<u64, AppError> {
    let stale: Vec<String> = sqlx::query_scalar!(
        "select storage_key from photo_pending_deletes
         where queued_at < now() - interval '7 days'"
    )
    .fetch_all(pool)
    .await?;
    if stale.is_empty() {
        return Ok(0);
    }
    storage.delete_objects(&stale).await?;
    let n = sqlx::query!(
        "delete from photo_pending_deletes where queued_at < now() - interval '7 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(n)
}

async fn purge_one_user(
    pool: &PgPool,
    storage: &dyn Storage,
    user_id: Uuid,
) -> Result<(), AppError> {
    // Collect S3 keys from photos owned by this user.
    let photo_keys: Vec<String> = sqlx::query_scalar!(
        "select storage_key from photos where owner_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    // Collect S3 keys for all thumbnails belonging to those photos.
    let thumb_keys: Vec<String> = sqlx::query_scalar!(
        "select t.storage_key from thumbnails t
         join photos p on p.id = t.photo_id
         where p.owner_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    let to_delete: Vec<String> = photo_keys.into_iter().chain(thumb_keys).collect();

    if !to_delete.is_empty() {
        storage.delete_objects(&to_delete).await?;
    }

    // Delete the user row. ON DELETE CASCADE removes photos, sessions,
    // oauth_identities, appreciations, follows, and tokens automatically.
    // Comments use ON DELETE SET NULL (pseudonymisation — body is preserved).
    sqlx::query!("delete from users where id = $1", user_id)
        .execute(pool)
        .await?;

    Ok(())
}
