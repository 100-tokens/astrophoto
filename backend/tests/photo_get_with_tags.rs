mod common;

use common::TestApp;

#[tokio::test]
async fn get_photo_returns_tags() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let photo_id = app.ready_photo(user_id).await;

    app.attach_tags(photo_id, &["andromeda", "galaxy"]).await;

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json(
            "GET",
            &format!("/api/photos/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 200);
    let mut tags: Vec<&str> = body["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    tags.sort();
    assert_eq!(tags, vec!["andromeda", "galaxy"]);
}

#[tokio::test]
async fn get_photo_returns_empty_tags_when_none() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app
        .signup_with_handle("Marie", "marie", "marie@example.com")
        .await;
    let photo_id = app.ready_photo(user_id).await;

    let (status, body): (_, serde_json::Value) = app
        .oneshot_json(
            "GET",
            &format!("/api/photos/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}
