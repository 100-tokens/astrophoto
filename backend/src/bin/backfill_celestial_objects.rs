//! One-shot identifier for already-solved photos. Idempotent.
//! Run via `just backfill-celestial-objects [--apply] [--reidentify]`.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let apply = std::env::args().any(|a| a == "--apply");
    let reidentify = std::env::args().any(|a| a == "--reidentify");

    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("APP_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL or APP_DATABASE_URL must be set"))?;
    let pool = PgPool::connect(&database_url).await?;

    let candidates: Vec<Uuid> = sqlx::query_scalar(
        r#"select id from photos
             where ra_deg is not null
               and ( $1::bool
                  or not exists (select 1 from photo_targets pt
                                  where pt.photo_id = photos.id
                                    and pt.source = 'plate_solve'))"#,
    )
    .bind(reidentify)
    .fetch_all(&pool)
    .await?;

    tracing::info!(total = candidates.len(), apply, reidentify, "backfill starting");

    let mut totals = (0_usize, 0_usize, 0_usize);
    for (i, id) in candidates.iter().enumerate() {
        if apply {
            let mut tx = pool.begin().await?;
            match astrophoto::celestial::identify(*id, &mut tx).await {
                Ok(out) => {
                    tx.commit().await?;
                    totals.0 += out.found;
                    totals.1 += out.kept;
                    totals.2 += out.dropped;
                }
                Err(e) => {
                    tracing::warn!(?id, error = %e, "identify failed; skipping");
                    tx.rollback().await?;
                }
            }
        }
        if (i + 1) % 50 == 0 {
            tracing::info!(processed = i + 1, ?totals, "progress");
        }
    }
    tracing::info!(
        found = totals.0,
        kept = totals.1,
        dropped = totals.2,
        apply,
        "backfill complete"
    );
    if !apply {
        tracing::warn!("DRY RUN — re-run with --apply to actually write");
    }
    Ok(())
}
