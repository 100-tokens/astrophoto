mod common;

use astrophoto::api_types::CategoryPage;
use axum::http::StatusCode;
use common::TestApp;

/// Happy-path: category page returns photos with matching category.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn category_page_returns_photos_for_category() {
    let app = TestApp::launch().await;
    let (_, uid) = app
        .signup_with_handle("Alice", "alice", "alice@x.test")
        .await;
    let p_dso = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;
    let _p_other = app.ready_photo_with(uid, "BBBB0001", Some("Moon")).await;

    // Set the category on the dso photo via non-macro query.
    sqlx::query("update photos set category = 'dso' where id = $1")
        .bind(p_dso)
        .execute(&app.pool)
        .await
        .unwrap();

    let (status, body) = app
        .oneshot_json::<CategoryPage>("GET", "/api/categories/dso", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.category, "dso");
    assert_eq!(body.photo_count, 1, "only one photo has category=dso");
    assert_eq!(body.page.photos.len(), 1);
    assert_eq!(body.page.photos[0].id, p_dso);
}

/// 404 for an invalid category value.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn category_404_for_invalid_category() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/categories/notarealcategory", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
