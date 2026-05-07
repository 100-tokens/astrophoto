mod common;

use common::TestApp;
use uuid::Uuid;

#[allow(clippy::unwrap_used)]
async fn insert_draft(pool: &sqlx::PgPool, owner_id: Uuid, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
         values ($1, $2, 'k', 'frame.fits', 1, 'image/jpeg', $3, $4, now())",
        id,
        owner_id,
        status,
        &id.simple().to_string()[..8]
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn drafts_list_returns_only_callers_drafts() {
    let app = TestApp::launch().await;
    let (cookie_a, user_a) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let (_, user_b) = app
        .signup_with_handle("Bob", "bob", "bob@example.com")
        .await;

    let _draft_a1 = insert_draft(&app.pool, user_a, "ready").await;
    let _draft_a2 = insert_draft(&app.pool, user_a, "processing").await;
    let _draft_b1 = insert_draft(&app.pool, user_b, "ready").await;

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json("GET", "/api/photos/me/drafts", Some(&cookie_a), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn drafts_list_excludes_published_photos() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let _draft = insert_draft(&app.pool, user_id, "ready").await;
    let _published = app.ready_photo(user_id).await; // ready_photo sets published_at

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json("GET", "/api/photos/me/drafts", Some(&cookie), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn drafts_list_requires_auth() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/photos/me/drafts", None, None)
        .await;
    assert_eq!(status, 401);
}
