//! Migration smoke tests. Each new schema bump gets a check that the
//! migrations apply cleanly to a fresh DB and the new schema objects
//! exist with the expected names.

use sqlx::Row;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;

async fn fresh_db() -> sqlx::PgPool {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // Hold container alive for the test scope by storing in a static
    // (testcontainers leaks otherwise on early return). Acceptable
    // for tests; not for prod.
    Box::leak(Box::new(pg));
    pool
}

#[tokio::test]
async fn migration_0005_adds_handles_and_redirects() {
    let pool = fresh_db().await;

    // users.handle exists, NOT NULL, unique
    let row = sqlx::query(
        "select column_name, is_nullable
           from information_schema.columns
          where table_name = 'users' and column_name = 'handle'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let nullable: String = row.try_get("is_nullable").unwrap();
    assert_eq!(nullable, "NO");

    // handle_redirects table exists
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.tables \
         where table_name = 'handle_redirects')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}

#[tokio::test]
async fn migration_0006_adds_user_tier() {
    let pool = fresh_db().await;
    let default_tier: String = sqlx::query_scalar(
        "select column_default \
         from information_schema.columns \
         where table_name = 'users' and column_name = 'tier'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(default_tier.starts_with("'free'"));
}

#[tokio::test]
async fn migration_0007_adds_photo_short_id() {
    let pool = fresh_db().await;
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.columns \
         where table_name = 'photos' and column_name = 'short_id')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}

#[tokio::test]
async fn migration_0008_adds_user_profile_fields() {
    let pool = fresh_db().await;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.columns \
         where table_name = 'users' \
         and column_name in ('tagline','bio_html','cover_photo_id', \
             'equipment_telescope','equipment_camera','equipment_mount', \
             'equipment_filters','equipment_guiding', \
             'location_text','bortle_class','sqm','social_links')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 12);
}

#[tokio::test]
async fn migration_0009_adds_photo_featured_and_category() {
    let pool = fresh_db().await;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.columns \
         where table_name = 'photos' \
         and column_name in ('featured_at','featured_position','category', \
             'scope','mount','filters','guiding')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 7);
}

#[tokio::test]
async fn migration_0010_adds_targets_and_tags() {
    let pool = fresh_db().await;
    let messier_count: i64 = sqlx::query_scalar(
        "select count(*) from targets where kind = 'messier'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(messier_count, 110);

    let m31: String = sqlx::query_scalar(
        "select canonical_name from targets where slug = 'm31'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(m31, "Andromeda Galaxy");
}
