mod common;

use astrophoto::api_types::TargetPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn target_page_returns_meta_plus_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;

    // Insert a test target row + the join (non-macro queries so they don't need the .sqlx cache).
    // Use a unique test slug to avoid collision with migration seed data.
    sqlx::query(
        "insert into targets (slug, canonical_name, aliases, kind) \
         values ('test-m31', 'M31', '{Andromeda Galaxy,NGC 224}', 'messier')",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let target_id: uuid::Uuid =
        sqlx::query_scalar("select id from targets where slug = 'test-m31'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    sqlx::query(
        "insert into photo_targets (photo_id, target_id, source) values ($1, $2, 'manual')",
    )
    .bind(p1)
    .bind(target_id)
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body) = app
        .oneshot_json::<TargetPage>("GET", "/api/targets/test-m31", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.target.slug, "test-m31");
    assert_eq!(body.target.canonical_name, "M31");
    assert_eq!(body.target.kind.as_deref(), Some("messier"));
    assert_eq!(
        body.target.aliases,
        vec!["Andromeda Galaxy", "NGC 224"]
    );
    assert_eq!(body.target.photo_count, 1);
    assert_eq!(body.target.contributor_count, 1);
    assert_eq!(body.page.photos.len(), 1);
    assert_eq!(body.page.photos[0].id, p1);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn target_404_for_unknown_slug() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/targets/notathing", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
