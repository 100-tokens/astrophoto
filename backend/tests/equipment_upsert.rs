//! Integration tests for the equipment_items upsert helper.
//! Uses an ephemeral Postgres via testcontainers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use astrophoto::equipment::upsert::{recompute_usage, upsert};
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use uuid::Uuid;

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

/// Create a throwaway user row and return its id. Tests need a real
/// users.id because `equipment_items.submitted_by` FK-references it.
async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let suffix = Uuid::new_v4().simple().to_string();
    let id: Uuid = sqlx::query_scalar(
        "insert into users (email, display_name, handle, password_hash, email_verified_at) \
         values ($1, 'Test', $2, 'x', now()) returning id",
    )
    .bind(format!("u-{suffix}@test.local"))
    .bind(format!("u-{}", &suffix[..8]))
    .fetch_one(pool)
    .await
    .unwrap();
    id
}

/// Empty or whitespace-only input returns Ok and writes nothing.
#[tokio::test]
async fn empty_input_writes_nothing() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    upsert(&pool, "camera", "", user_id).await.unwrap();
    upsert(&pool, "camera", "   ", user_id).await.unwrap();

    let count: i64 = sqlx::query_scalar("select count(*) from equipment_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

/// Two calls with the same display string produce a single row.
/// `usage_count` stays at 0 until `recompute_usage` is called — the
/// in-place +1 bump on the upsert path was removed in May 2026 because
/// re-saving the same photo overcounted; counting is now derived from
/// actual photo references.
#[tokio::test]
async fn same_display_is_idempotent() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    upsert(&pool, "telescope", "Celestron EdgeHD 8", user_id)
        .await
        .unwrap();
    upsert(&pool, "telescope", "Celestron EdgeHD 8", user_id)
        .await
        .unwrap();

    let (count, usage): (i64, i32) = sqlx::query_as(
        "select count(*), max(usage_count) from equipment_items where kind = 'telescope'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "should be exactly one row");
    assert_eq!(
        usage, 0,
        "usage_count should NOT auto-bump on upsert; recompute drives it"
    );
}

/// Two calls with the same canonical but different casing collapse to
/// one row, and `display_name` preserves the first-seen casing.
#[tokio::test]
async fn canonical_match_preserves_first_seen_display() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    // First call — mixed case, this sets display_name.
    upsert(&pool, "camera", "ZWO ASI2600MC", user_id)
        .await
        .unwrap();
    // Second call — lowercase canonical match, but different display form.
    upsert(&pool, "camera", "zwo asi2600mc", user_id)
        .await
        .unwrap();

    let (count, display): (i64, String) = sqlx::query_as(
        "select count(*), max(display_name) \
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
}

/// Internal whitespace runs collapse to a single space at canonical
/// time, so `"Sky-Watcher  Esprit 100 ED"` (double space) maps to the
/// same row as the single-space form.
#[tokio::test]
async fn internal_whitespace_collapses() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    upsert(&pool, "telescope", "Sky-Watcher  Esprit 100 ED", user_id)
        .await
        .unwrap();
    upsert(&pool, "telescope", "Sky-Watcher Esprit 100 ED", user_id)
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar("select count(*) from equipment_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "double-space variant must collapse to one row");
}

/// New rows are auto-approved and must carry an approved_at timestamp,
/// plus the calling user recorded as submitter.
#[tokio::test]
async fn new_row_is_auto_approved_with_timestamp() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    upsert(&pool, "filter", "Astronomik OIII 6nm", user_id)
        .await
        .unwrap();

    let row = sqlx::query!(
        "select status, approved_at, submitted_by from equipment_items \
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
    assert_eq!(
        row.submitted_by,
        Some(user_id),
        "the calling user should be recorded as submitter"
    );
}

/// `recompute_usage` reflects the count of distinct photos that reference
/// the item via freetext columns. Two photos with the same `camera`
/// display_name → usage_count = 2.
#[tokio::test]
async fn recompute_counts_distinct_photos_via_freetext() {
    let pool = fresh_db().await;
    let user_id = make_user(&pool).await;

    upsert(&pool, "camera", "ZWO ASI2600MC", user_id)
        .await
        .unwrap();
    let item_id: Uuid = sqlx::query_scalar(
        "select id from equipment_items where kind='camera' \
         and canonical_name='zwo asi2600mc'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Insert two photo rows owned by user_id with that camera. Minimal
    // columns — the rest of the schema's NULLable columns can default.
    for _ in 0..2 {
        sqlx::query(
            "insert into photos (id, owner_id, short_id, storage_key, original_name, \
                                  bytes, mime, status, camera, original_uploaded_at) \
             values (gen_random_uuid(), $1, substr(md5(random()::text),1,8), \
                     'x', 'x.jpg', 0, 'image/jpeg', 'ready', 'ZWO ASI2600MC', now())",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    recompute_usage(&pool, item_id).await.unwrap();

    let usage: i32 = sqlx::query_scalar("select usage_count from equipment_items where id = $1")
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(usage, 2, "two distinct photos → usage_count = 2");
}
