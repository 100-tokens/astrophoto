#![allow(clippy::unwrap_used)]
mod common;

use common::TestApp;
use serde_json::json;
use uuid::Uuid;

async fn insert_with_status(pool: &sqlx::PgPool, owner: Uuid, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
         values ($1, $2, 'k', 'f', 1, 'image/jpeg', $3, $4, now())",
        id, owner, status, &id.simple().to_string()[..8]
    ).execute(pool).await.unwrap();
    id
}

#[tokio::test]
async fn batch_publish_publishes_ready_skips_processing() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("M", "marie", "m@m.com").await;
    let ready = insert_with_status(&app.pool, user_id, "ready").await;
    let processing = insert_with_status(&app.pool, user_id, "processing").await;
    let failed = insert_with_status(&app.pool, user_id, "failed").await;

    let body = json!({ "ids": [ready, processing, failed] });
    let (status, resp): (_, serde_json::Value) = app
        .oneshot_json(
            "POST",
            "/api/photos/batch/publish",
            Some(&cookie),
            Some(body),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(resp["published"].as_array().unwrap().len(), 1);
    assert_eq!(resp["skipped"].as_array().unwrap().len(), 2);

    let published_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar!("select published_at from photos where id = $1", ready)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(published_at.is_some());
}

#[tokio::test]
async fn batch_publish_403_on_other_users_id() {
    let app = TestApp::launch().await;
    let (cookie_a, user_a) = app.signup_with_handle("A", "alice", "a@a.com").await;
    let (_, user_b) = app.signup_with_handle("B", "bob", "b@b.com").await;
    let mine = insert_with_status(&app.pool, user_a, "ready").await;
    let theirs = insert_with_status(&app.pool, user_b, "ready").await;

    let (status, _) = app
        .oneshot(
            "POST",
            "/api/photos/batch/publish",
            Some(&cookie_a),
            Some(json!({ "ids": [mine, theirs] })),
        )
        .await;
    assert_eq!(status, 403);
}

#[tokio::test]
async fn batch_publish_already_published_is_skipped_not_errored() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("M", "marie", "m@m.com").await;
    let pub1 = app.ready_photo(user_id).await;

    let (status, resp): (_, serde_json::Value) = app
        .oneshot_json(
            "POST",
            "/api/photos/batch/publish",
            Some(&cookie),
            Some(json!({ "ids": [pub1] })),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(resp["skipped"][0]["reason"], "already_published");
}
