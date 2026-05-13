mod common;

use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn me_returns_tier_free_for_new_user() {
    let app = TestApp::launch().await;
    let (cookie, _) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["tier"], json!("free"));
}

#[tokio::test]
async fn me_returns_tier_subscriber_after_upgrade() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    sqlx::query!(
        "update users set tier = 'subscriber' where id = $1",
        user_id
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["tier"], json!("subscriber"));
}
