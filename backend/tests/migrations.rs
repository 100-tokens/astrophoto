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
