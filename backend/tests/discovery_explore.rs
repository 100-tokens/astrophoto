mod common;

use astrophoto::api_types::DiscoveryPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_returns_published_photos_newest_first_across_authors() {
    let app = TestApp::launch().await;
    let (_, alice_id) = app.signup_with_handle("Alice", "alice", "alice@x.test").await;
    let (_, bob_id) = app.signup_with_handle("Bob", "bob", "bob@x.test").await;
    let _p1 = app.ready_photo_with(alice_id, "AAAA0001", Some("M31")).await;
    let _p2 = app.ready_photo_with(bob_id, "BBBB0001", Some("M42")).await;
    let p3 = app.ready_photo_with(alice_id, "AAAA0002", Some("NGC 7000")).await;

    let (status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=2", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.photos.len(), 2);
    assert_eq!(body.photos[0].id, p3, "newest first across owners");
    assert!(body.next_cursor.is_some(), "more pages remain");
    // Author chip data must come back.
    assert_eq!(body.photos[0].author_handle, "alice");
    assert_eq!(body.photos[0].author_display_name, "Alice");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_respects_limit_clamp() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/explore?limit=999", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.oneshot("GET", "/api/explore?limit=0", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_filters_by_category() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let p_dso = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;
    let _p_lunar = app.ready_photo_with(uid, "BBBB0001", Some("Moon")).await;
    sqlx::query!("update photos set category = 'dso' where id = $1", p_dso)
        .execute(&app.pool).await.unwrap();
    sqlx::query!("update photos set category = 'lunar' where id = $1", _p_lunar)
        .execute(&app.pool).await.unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?category=dso", None, None)
        .await;
    assert_eq!(body.photos.len(), 1);
    assert_eq!(body.photos[0].id, p_dso);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_sort_most_appreciated_orders_by_count() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    sqlx::query!("update photos set appreciations_count = 5 where id = $1", p1)
        .execute(&app.pool).await.unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?sort=most-appreciated", None, None)
        .await;
    assert_eq!(body.photos[0].id, p1);
    assert_eq!(body.photos[1].id, p2);
}
