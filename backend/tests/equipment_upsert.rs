//! Integration tests for the equipment_items upsert helper.
//! Uses an ephemeral Postgres via testcontainers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use astrophoto::equipment::upsert::upsert;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;

async fn fresh_db() -> sqlx::PgPool {
    let pg = PgImage::default()
        .with_tag("16-alpine")
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // Hold container alive for the test scope. Acceptable for tests.
    Box::leak(Box::new(pg));
    pool
}

/// Empty or whitespace-only input returns Ok and writes nothing.
#[tokio::test]
async fn empty_input_writes_nothing() {
    let pool = fresh_db().await;

    upsert(&pool, "camera", "").await.unwrap();
    upsert(&pool, "camera", "   ").await.unwrap();

    let count: i64 = sqlx::query_scalar("select count(*) from equipment_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

/// Two calls with the same display string produce one row with usage_count=2.
#[tokio::test]
async fn same_display_increments_count() {
    let pool = fresh_db().await;

    upsert(&pool, "telescope", "Celestron EdgeHD 8")
        .await
        .unwrap();
    upsert(&pool, "telescope", "Celestron EdgeHD 8")
        .await
        .unwrap();

    let (count, usage): (i64, i32) = sqlx::query_as(
        "select count(*), max(usage_count) from equipment_items where kind = 'telescope'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "should be exactly one row");
    assert_eq!(usage, 2, "usage_count should be 2");
}

/// Two calls with the same canonical but different casing produce one row
/// with usage_count=2 and display_name preserved from the first insert.
#[tokio::test]
async fn canonical_match_preserves_first_seen_display() {
    let pool = fresh_db().await;

    // First call — mixed case, this sets display_name.
    upsert(&pool, "camera", "ZWO ASI2600MC").await.unwrap();
    // Second call — lowercase canonical match, but different display form.
    upsert(&pool, "camera", "zwo asi2600mc").await.unwrap();

    let (count, display, usage): (i64, String, i32) = sqlx::query_as(
        "select count(*), max(display_name), max(usage_count) \
         from equipment_items where kind = 'camera'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "should be exactly one row");
    assert_eq!(
        display, "ZWO ASI2600MC",
        "display_name should be first-seen"
    );
    assert_eq!(usage, 2, "usage_count should be 2");
}

/// New rows are auto-approved and must carry an approved_at timestamp.
#[tokio::test]
async fn new_row_is_auto_approved_with_timestamp() {
    let pool = fresh_db().await;

    upsert(&pool, "filter", "Astronomik OIII 6nm")
        .await
        .unwrap();

    let row = sqlx::query!(
        "select status, approved_at from equipment_items \
         where kind = 'filter' and canonical_name = 'astronomik oiii 6nm'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.status, "approved");
    assert!(
        row.approved_at.is_some(),
        "upsert must stamp approved_at on insert so the UI never shows \
         '—' for auto-approved items"
    );
}
