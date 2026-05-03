mod common;

use astrophoto::api_types::GalleryPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn photos_feed_returns_published_photos_newest_first() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let _p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let _p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    let p3 = app.ready_photo_with(uid, "CCCC0003", None).await;

    let (status, body) = app
        .oneshot_json::<GalleryPage>(
            "GET",
            "/api/users/by-handle/marie/photos?limit=2",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.photos.len(), 2);
    assert_eq!(body.photos[0].id, p3, "newest first");
    assert!(body.next_cursor.is_some());

    let cursor = body.next_cursor.unwrap();
    let (status, page2) = app
        .oneshot_json::<GalleryPage>(
            "GET",
            &format!("/api/users/by-handle/marie/photos?limit=2&cursor={cursor}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page2.photos.len(), 1);
    assert!(page2.next_cursor.is_none());
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn photos_feed_404_for_unknown_handle() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/users/by-handle/nobody/photos", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn photos_feed_respects_limit_bounds() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let _ = app.ready_photo_with(uid, "AAAA0001", None).await;

    let (status, _) = app
        .oneshot(
            "GET",
            "/api/users/by-handle/marie/photos?limit=999",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "limit clamped, not 400");

    let (status, _) = app
        .oneshot(
            "GET",
            "/api/users/by-handle/marie/photos?limit=0",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn photos_feed_sort_popular_orders_by_appreciations() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;

    sqlx::query!(
        "update photos set appreciations_count = 5 where id = $1",
        p1
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (_status, body) = app
        .oneshot_json::<GalleryPage>(
            "GET",
            "/api/users/by-handle/marie/photos?sort=popular",
            None,
            None,
        )
        .await;
    assert_eq!(
        body.photos[0].id, p1,
        "popular puts highest appreciations first"
    );
    assert_eq!(body.photos[1].id, p2);
}
