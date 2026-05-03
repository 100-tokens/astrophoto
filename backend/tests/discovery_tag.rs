mod common;

use astrophoto::api_types::TagPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn tag_page_returns_meta_plus_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;

    // Insert the tag row + the join (non-macro queries so they don't need the .sqlx cache).
    sqlx::query("insert into tags (slug, name) values ('deepsky', 'Deep Sky')")
        .execute(&app.pool)
        .await
        .unwrap();
    let tag_id: uuid::Uuid = sqlx::query_scalar("select id from tags where slug = 'deepsky'")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    sqlx::query("insert into photo_tags (photo_id, tag_id) values ($1, $2)")
        .bind(p1)
        .bind(tag_id)
        .execute(&app.pool)
        .await
        .unwrap();

    let (status, body) = app
        .oneshot_json::<TagPage>("GET", "/api/tags/deepsky", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.tag.slug, "deepsky");
    assert_eq!(body.tag.name, "Deep Sky");
    assert_eq!(body.tag.photo_count, 1);
    assert_eq!(body.page.photos.len(), 1);
    assert_eq!(body.page.photos[0].id, p1);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn tag_404_for_unknown_slug() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/tags/notathing", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
