#![allow(clippy::unwrap_used)]

mod common;

use common::TestApp;
use uuid::Uuid;

async fn insert_pending(pool: &sqlx::PgPool, owner_id: Uuid, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
         values ($1, $2, $3, 'frame.fits', 1, 'image/jpeg', $4, $5, now())",
        id,
        owner_id,
        format!("originals/{id}"),
        status,
        &id.simple().to_string()[..8]
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn cancel_pending_deletes_row() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let photo_id = insert_pending(&app.pool, user_id, "pending").await;

    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/uploads/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 204);

    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from photos where id = $1",
        photo_id
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn cancel_processing_draft_is_allowed() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let photo_id = insert_pending(&app.pool, user_id, "processing").await;

    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/uploads/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 204);
}

#[tokio::test]
async fn cancel_published_photo_is_rejected() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let photo_id = app.ready_photo(user_id).await; // published

    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/uploads/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 409); // Conflict: not in cancellable state
}

#[tokio::test]
async fn cancel_other_users_photo_returns_403() {
    let app = TestApp::launch().await;
    let (_, owner) = app
        .signup_with_handle("Owner", "owner", "owner@example.com")
        .await;
    let (cookie_other, _) = app
        .signup_with_handle("Other", "other", "other@example.com")
        .await;
    let photo_id = insert_pending(&app.pool, owner, "pending").await;

    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/uploads/{photo_id}"),
            Some(&cookie_other),
            None,
        )
        .await;
    assert_eq!(status, 403);
}
