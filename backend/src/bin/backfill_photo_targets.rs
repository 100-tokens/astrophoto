//! One-shot: for photos with `target` text but no manual photo_targets row,
//! resolve the text against the catalog and insert the primary join row.
//!
//! Default: dry-run (prints counts only).
//! Pass `--apply` to write rows. Idempotent — a second `--apply` is a no-op.

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Backfill manual photo_targets rows from photos.target text.")]
struct Args {
    /// Without this flag, runs in dry-run mode and only prints counts.
    #[arg(long, default_value_t = false)]
    apply: bool,
}

#[derive(Default, Debug)]
pub struct BackfillCounts {
    pub eligible_photos: usize,
    pub matched: usize,
    pub no_match: usize,
}

pub async fn run_once(pool: &sqlx::PgPool, apply: bool) -> Result<BackfillCounts> {
    let mut counts = BackfillCounts::default();

    let rows = sqlx::query!(
        r#"
        select p.id as "id!", p.target as "target!"
        from photos p
        where p.target is not null and trim(p.target) <> ''
          and not exists (
            select 1 from photo_targets pt
             where pt.photo_id = p.id and pt.source = 'manual'
          )
        "#
    )
    .fetch_all(pool)
    .await?;

    counts.eligible_photos = rows.len();

    for row in rows {
        let trimmed = row.target.trim().to_owned();
        // Normalise: remove whitespace for slug comparison ("M 31" → "m31").
        let normalised = trimmed
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase();

        let target_id: Option<sqlx::types::Uuid> = sqlx::query_scalar!(
            r#"
            select id from targets
             where slug = $1
                or $2 = any (aliases)
                or canonical_name ilike $2
             limit 1
            "#,
            normalised,
            trimmed
        )
        .fetch_optional(pool)
        .await?;

        match target_id {
            Some(tid) => {
                counts.matched += 1;
                if apply {
                    sqlx::query!(
                        "insert into photo_targets (photo_id, target_id, source, is_primary) \
                         values ($1, $2, 'manual', true) \
                         on conflict (photo_id, target_id) do update set is_primary=true, source='manual'",
                        row.id,
                        tid
                    )
                    .execute(pool)
                    .await?;
                }
            }
            None => counts.no_match += 1,
        }
    }

    Ok(counts)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let url = std::env::var("DATABASE_URL").or_else(|_| std::env::var("APP_DATABASE_URL"))?;
    let pool = sqlx::PgPool::connect(&url).await?;
    let counts = run_once(&pool, args.apply).await?;
    tracing::info!(
        eligible_photos = counts.eligible_photos,
        matched = counts.matched,
        no_match = counts.no_match,
        apply = args.apply,
        "backfill-photo-targets complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use sqlx::PgPool;
    use testcontainers::ImageExt;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres as PgImage;
    use uuid::Uuid;

    async fn fresh_pool() -> (PgPool, testcontainers::ContainerAsync<PgImage>) {
        let pg = PgImage::default()
            .with_tag("16-alpine")
            .start()
            .await
            .unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, pg)
    }

    /// Insert a minimal user + photo. Returns `photo_id`.
    async fn seed_photo(pool: &PgPool, target_text: &str) -> Uuid {
        // Each call uses a unique suffix so email/handle constraints don't collide.
        let suffix = Uuid::new_v4().to_string().replace('-', "");
        let short = &suffix[..8];
        let email = format!("user-{}@test.local", short);
        let handle = format!("u-{}", short);
        let user_id: Uuid = sqlx::query_scalar!(
            "insert into users (email, display_name, handle) \
             values ($1, 'Test User', $2) returning id",
            email,
            handle
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let photo_id = Uuid::new_v4();
        let short_id = Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect::<String>()
            .to_uppercase();
        sqlx::query!(
            "insert into photos \
             (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, target, original_uploaded_at, published_at) \
             values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', $3, $4, now(), now())",
            photo_id,
            user_id,
            short_id,
            target_text
        )
        .execute(pool)
        .await
        .unwrap();

        photo_id
    }

    #[tokio::test]
    async fn dry_run_does_not_write() {
        let (pool, _c) = fresh_pool().await;
        let _photo_id = seed_photo(&pool, "M 31").await;
        let counts = run_once(&pool, false).await.unwrap();
        assert!(counts.matched >= 1, "M 31 should match m31");
        let manual_rows: i64 = sqlx::query_scalar!(
            "select count(*) as \"c!\" from photo_targets where source='manual'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(manual_rows, 0, "dry-run must not write");
    }

    #[tokio::test]
    async fn apply_writes_primary_row() {
        let (pool, _c) = fresh_pool().await;
        let photo_id = seed_photo(&pool, "M 31").await;
        let counts = run_once(&pool, true).await.unwrap();
        assert!(counts.matched >= 1);
        let rows = sqlx::query!(
            r#"
            select t.slug as "slug!", pt.is_primary
            from photo_targets pt
            join targets t on t.id = pt.target_id
            where pt.photo_id = $1 and pt.source = 'manual'
            "#,
            photo_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].slug, "m31");
        assert!(rows[0].is_primary);
    }

    #[tokio::test]
    async fn second_apply_is_noop() {
        let (pool, _c) = fresh_pool().await;
        let _photo_id = seed_photo(&pool, "Andromeda Galaxy").await;
        run_once(&pool, true).await.unwrap();
        let counts2 = run_once(&pool, true).await.unwrap();
        // The eligibility WHERE clause excludes photos that already have a manual row,
        // so the second pass sees zero eligible photos.
        assert_eq!(counts2.eligible_photos, 0);
        assert_eq!(counts2.matched, 0);
    }

    #[tokio::test]
    async fn unmatched_target_counts_no_match() {
        let (pool, _c) = fresh_pool().await;
        let _photo_id = seed_photo(&pool, "Total nonsense gibberish").await;
        let counts = run_once(&pool, true).await.unwrap();
        assert_eq!(counts.eligible_photos, 1);
        assert_eq!(counts.matched, 0);
        assert_eq!(counts.no_match, 1);
        let manual_rows: i64 = sqlx::query_scalar!(
            "select count(*) as \"c!\" from photo_targets where source='manual'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(manual_rows, 0);
    }
}
