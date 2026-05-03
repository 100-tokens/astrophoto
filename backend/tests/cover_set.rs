mod common;

use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn set_cover_writes_users_cover_photo_id() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;
    let photo_id = app.ready_photo(user_id).await;

    let body = serde_json::json!({ "photo_id": photo_id });
    let (status, _) = app.oneshot("POST", "/api/me/cover", Some(&cookie), Some(body)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let row = sqlx::query!("select cover_photo_id from users where id = $1", user_id)
        .fetch_one(&app.pool).await.unwrap();
    assert_eq!(row.cover_photo_id, Some(photo_id));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn set_cover_404_when_photo_not_owned() {
    let app = TestApp::launch().await;
    let (a_cookie, _a) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let (_b_cookie, b_id) = app.signup_with_handle("B", "bob", "b@x.test").await;
    let other = app.ready_photo(b_id).await;

    let body = serde_json::json!({ "photo_id": other });
    let (status, _) = app.oneshot("POST", "/api/me/cover", Some(&a_cookie), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn set_cover_clears_when_photo_id_null() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;
    let photo_id = app.ready_photo(user_id).await;

    app.oneshot(
        "POST",
        "/api/me/cover",
        Some(&cookie),
        Some(serde_json::json!({ "photo_id": photo_id })),
    ).await;

    let body = serde_json::json!({ "photo_id": Option::<Uuid>::None });
    let (status, _) = app.oneshot("POST", "/api/me/cover", Some(&cookie), Some(body)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let row = sqlx::query!("select cover_photo_id from users where id = $1", user_id)
        .fetch_one(&app.pool).await.unwrap();
    assert_eq!(row.cover_photo_id, None);
}
