use std::sync::Arc;

use astrophoto::{db, jobs::orphan_reaper, storage::MemoryStorage};
use bytes::Bytes;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a `photos` row in `status='pending'` with the given `created_at`
/// offset so the reaper's threshold can be tested. Returns the photo id.
#[allow(clippy::unwrap_used)]
async fn insert_pending_photo(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    storage_key: &str,
    created_at_expr: &str, // e.g. "now() - interval '3 hours'"
    short_id: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    // Use a raw query with the literal timestamp expression so we can back-date
    // `created_at` and `original_uploaded_at` without binding limitations.
    let sql = format!(
        r#"
        insert into photos
            (id, owner_id, storage_key, original_name, bytes, mime,
             short_id, status, last_step,
             created_at, original_uploaded_at)
        values ('{id}', '{owner_id}', '{storage_key}', 'test.jpg', 1000, 'image/jpeg',
                '{short_id}', 'pending', 'upload',
                {created_at_expr}, {created_at_expr})
        "#
    );
    sqlx::query(&sql).execute(pool).await.unwrap();
    id
}

// ---------------------------------------------------------------------------
// Test 1: stale pending photo (3 hours old) is reaped
// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn reaper_deletes_stale_pending_photos() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Insert a user so the FK constraint is satisfied.
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle) \
         values ($1, 'reaper1@example.com', 'x', 'Reaper1', 'reaper1')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let key = "originals/stale-pending-test";

    let photo_id = insert_pending_photo(
        &pool,
        user_id,
        key,
        "now() - interval '3 hours'",
        "STALE001",
    )
    .await;

    // Put a fake payload at the storage key so we can confirm deletion.
    storage
        .put(key, "image/jpeg", Bytes::from_static(&[0u8; 4]))
        .await
        .unwrap();

    // Run one sweep.
    orphan_reaper::sweep_once(&pool, storage.as_ref())
        .await
        .unwrap();

    // Row must be gone.
    let exists: bool = sqlx::query_scalar!(
        "select exists(select 1 from photos where id = $1)",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(false);
    assert!(!exists, "stale pending photo row should have been reaped");

    // Storage object must be gone too.
    let data = storage.get(key).await.unwrap();
    assert!(
        data.is_none(),
        "storage object should have been deleted by the reaper"
    );
}

// ---------------------------------------------------------------------------
// Test 2: recent pending photo (10 minutes old) is NOT reaped
// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn reaper_spares_recent_pending_photos() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let user_id = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle) \
         values ($1, 'reaper2@example.com', 'x', 'Reaper2', 'reaper2')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let key = "originals/recent-pending-test";

    let photo_id = insert_pending_photo(
        &pool,
        user_id,
        key,
        "now() - interval '10 minutes'",
        "RECENT01",
    )
    .await;

    storage
        .put(key, "image/jpeg", Bytes::from_static(&[0u8; 4]))
        .await
        .unwrap();

    // Run one sweep — the row is only 10 minutes old, under the 2-hour threshold.
    orphan_reaper::sweep_once(&pool, storage.as_ref())
        .await
        .unwrap();

    // Row must still be present.
    let exists: bool = sqlx::query_scalar!(
        "select exists(select 1 from photos where id = $1)",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(false);
    assert!(exists, "recent pending photo should NOT have been reaped");

    // Storage object must also still be present.
    let data = storage.get(key).await.unwrap();
    assert!(
        data.is_some(),
        "storage object should NOT have been deleted for a recent pending photo"
    );
}
