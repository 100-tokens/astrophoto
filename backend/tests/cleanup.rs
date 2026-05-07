#![allow(clippy::unwrap_used)]

mod common;

use astrophoto::photos::cleanup::reap_once;
use common::TestApp;
use uuid::Uuid;

async fn insert_pending_with_age(pool: &sqlx::PgPool, owner_id: Uuid, hours_ago: i32) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at, created_at) \
         values ($1, $2, $3, 'f', 1, 'image/jpeg', 'pending', $4, now() - ($5 || ' hours')::interval, now() - ($5 || ' hours')::interval)",
        id, owner_id, format!("originals/{id}"), &id.simple().to_string()[..8], hours_ago.to_string()
    ).execute(pool).await.unwrap();
    id
}

#[tokio::test]
async fn reap_deletes_pending_older_than_24h() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;

    let old = insert_pending_with_age(&app.pool, user_id, 25).await;
    let recent = insert_pending_with_age(&app.pool, user_id, 1).await;

    let storage = std::sync::Arc::new(astrophoto::storage::MemoryStorage::new());
    let count = reap_once(&app.pool, storage.as_ref()).await.unwrap();
    assert_eq!(count, 1);

    let count_old: i64 =
        sqlx::query_scalar!("select count(*) as \"c!\" from photos where id = $1", old)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    let count_recent: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from photos where id = $1",
        recent
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(count_old, 0);
    assert_eq!(count_recent, 1);
}

#[tokio::test]
async fn reap_does_not_delete_processing_or_published() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let published = app.ready_photo(user_id).await;

    let storage = std::sync::Arc::new(astrophoto::storage::MemoryStorage::new());
    let _ = reap_once(&app.pool, storage.as_ref()).await.unwrap();

    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from photos where id = $1",
        published
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}
