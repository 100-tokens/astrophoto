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

    // Bounded retention for the auth tables — nothing else ever deletes
    // expired sessions or spent one-shot tokens, so they grow forever.
    match purge_expired_auth_rows(pool).await {
        Ok((sessions, tokens)) if sessions > 0 || tokens > 0 => {
            tracing::info!(sessions, tokens, "purged expired sessions/auth tokens");
        }
        Ok(_) => {}
        Err(e) => tracing::error!(error = ?e, "auth-row purge failed"),
    }

    tracing::info!(deleted, total_due = due.len(), "purge cycle done");
    Ok(deleted)
}

/// Delete expired sessions (7 days past `expires_at`, cheap via
/// `sessions_expires_at_idx`) and month-old one-shot auth tokens.
///
/// The token deletes are purely age-based on `created_at` — NOT on
/// `used_at`/`expires_at` — because those tables double as rate-limit
/// logs: the issue paths count rows with `created_at > now() - interval
/// '1 hour'` (see password_reset / email_verify / email_change). All
/// tokens expire within hours, so 30-day-old rows are inert for both
/// auth and throttling; deleting them also drops stale IP and
/// pending-email PII.
pub async fn purge_expired_auth_rows(pool: &PgPool) -> Result<(u64, u64), AppError> {
    let sessions =
        sqlx::query!("delete from sessions where expires_at < now() - interval '7 days'")
            .execute(pool)
            .await?
            .rows_affected();
    let mut tokens = 0u64;
    tokens += sqlx::query!(
        "delete from email_verification_tokens where created_at < now() - interval '30 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    tokens += sqlx::query!(
        "delete from password_reset_tokens where created_at < now() - interval '30 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    tokens += sqlx::query!(
        "delete from email_change_tokens where created_at < now() - interval '30 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok((sessions, tokens))
}

pub async fn sweep_pending_deletes(pool: &PgPool, storage: &dyn Storage) -> Result<u64, AppError> {
    // Only drain rows whose photo is back in status='ready': pending
    // deletes hold the PREVIOUS original/thumbnails of a replace, and a
    // replace that failed (or is still mid-calibration) means those keys
    // are the only good assets the photo has left. Deleting them on age
    // alone would permanently destroy a bricked photo's archival master.
    let stale: Vec<String> = sqlx::query_scalar!(
        "select d.storage_key from photo_pending_deletes d
           join photos p on p.id = d.photo_id
          where d.queued_at < now() - interval '7 days'
            and p.status = 'ready'"
    )
    .fetch_all(pool)
    .await?;
    if stale.is_empty() {
        return Ok(0);
    }
    storage.delete_objects(&stale).await?;
    let n = sqlx::query!(
        "delete from photo_pending_deletes d
          using photos p
          where p.id = d.photo_id
            and d.queued_at < now() - interval '7 days'
            and p.status = 'ready'"
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

    // Display masters — the keys CloudFront actually serves. Skipping
    // them would leave every deleted user's images publicly fetchable.
    let display_keys: Vec<String> = sqlx::query_scalar!(
        r#"select display_key as "display_key!" from photos
           where owner_id = $1 and display_key is not null"#,
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

    // Keys still queued from replaces (the user CASCADE takes the rows).
    let pending_keys: Vec<String> = sqlx::query_scalar!(
        "select pd.storage_key from photo_pending_deletes pd
         join photos p on p.id = pd.photo_id
         where p.owner_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    // The avatar display object lives outside the photos table.
    let avatar_id: Option<Uuid> =
        sqlx::query_scalar!("select avatar_id from users where id = $1", user_id)
            .fetch_optional(pool)
            .await?
            .flatten();

    let mut to_delete: Vec<String> = photo_keys
        .into_iter()
        .chain(display_keys)
        .chain(thumb_keys)
        .chain(pending_keys)
        .collect();
    if let Some(aid) = avatar_id {
        to_delete.push(format!("display/{aid}.jpg"));
    }

    if !to_delete.is_empty() {
        storage.delete_objects(&to_delete).await?;
    }

    // The user CASCADE silently removes their `appreciations` rows, but
    // the denormalized photos.appreciations_count on OTHER users' photos
    // would keep the old totals forever (the drift class migration 0024
    // resynced once). Decrement in the same transaction as the delete;
    // greatest() mirrors the unappreciate handler's underflow guard.
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"update photos p
              set appreciations_count = greatest(p.appreciations_count - a.cnt, 0)
             from (select photo_id, count(*)::int4 as cnt
                     from appreciations
                    where user_id = $1
                    group by photo_id) a
            where p.id = a.photo_id"#,
        user_id
    )
    .execute(&mut *tx)
    .await?;

    // Delete the user row. ON DELETE CASCADE removes photos, sessions,
    // oauth_identities, appreciations, follows, and tokens automatically.
    // Comments use ON DELETE SET NULL (pseudonymisation — body is preserved).
    sqlx::query!("delete from users where id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(())
}
