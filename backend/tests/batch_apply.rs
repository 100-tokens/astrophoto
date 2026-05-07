#![allow(clippy::unwrap_used)]
mod common;

use common::TestApp;
use serde_json::json;
use uuid::Uuid;

async fn insert_draft(pool: &sqlx::PgPool, owner_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
         values ($1, $2, 'k', 'f', 1, 'image/jpeg', 'ready', $3, now())",
        id,
        owner_id,
        &id.simple().to_string()[..8]
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn batch_apply_sets_target_and_tags_on_all() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let id1 = insert_draft(&app.pool, user_id).await;
    let id2 = insert_draft(&app.pool, user_id).await;

    let body = json!({ "ids": [id1, id2], "target": "M31", "tags": ["andromeda"] });
    let (status, resp): (_, serde_json::Value) = app
        .oneshot_json("POST", "/api/photos/batch/apply", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, 200);
    assert_eq!(resp["applied"], json!(2));

    let target1: Option<String> =
        sqlx::query_scalar!("select target from photos where id = $1", id1)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(target1.as_deref(), Some("M31"));
}

#[tokio::test]
async fn batch_apply_rejects_other_users_ids() {
    let app = TestApp::launch().await;
    let (cookie_a, user_a) = app.signup_with_handle("A", "alice", "a@a.com").await;
    let (_, user_b) = app.signup_with_handle("B", "bob", "b@b.com").await;
    let mine = insert_draft(&app.pool, user_a).await;
    let theirs = insert_draft(&app.pool, user_b).await;

    let body = json!({ "ids": [mine, theirs], "target": "M31" });
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/photos/batch/apply",
            Some(&cookie_a),
            Some(body),
        )
        .await;
    assert_eq!(status, 403);
}

#[tokio::test]
async fn batch_apply_rejects_published_photos() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("M", "marie", "m@m.com").await;
    let draft = insert_draft(&app.pool, user_id).await;
    let published = app.ready_photo(user_id).await;

    let body = json!({ "ids": [draft, published], "target": "M31" });
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/photos/batch/apply",
            Some(&cookie),
            Some(body),
        )
        .await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn batch_apply_with_empty_string_target_is_400() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("M", "m", "m@m.com").await;
    let draft = insert_draft(&app.pool, user_id).await;

    let body = json!({ "ids": [draft], "target": "" });
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/photos/batch/apply",
            Some(&cookie),
            Some(body),
        )
        .await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn batch_apply_empty_tags_array_clears_tags() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("M", "m", "m@m.com").await;
    let draft = insert_draft(&app.pool, user_id).await;
    app.attach_tags(draft, &["old"]).await;

    let body = json!({ "ids": [draft], "tags": [] });
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/photos/batch/apply",
            Some(&cookie),
            Some(body),
        )
        .await;
    assert_eq!(status, 200);

    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from photo_tags where photo_id = $1",
        draft
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}
