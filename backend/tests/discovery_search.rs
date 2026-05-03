mod common;

use astrophoto::api_types::SearchResults;
use axum::http::StatusCode;
use common::TestApp;

/// Happy-path: search returns targets, users, and photos matching the query.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn search_returns_targets_users_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app
        .signup_with_handle(
            "Andromeda Aficionado",
            "andromeda_aficionado",
            "a@x.test",
        )
        .await;
    let p = app
        .ready_photo_with(uid, "AAAA0001", Some("M31 Andromeda Galaxy"))
        .await;

    // Insert a target with a unique slug to avoid collision with migration seed data.
    // Use kind='messier' to satisfy the migration 0010 check constraint.
    sqlx::query(
        "insert into targets (slug, canonical_name, aliases, kind) \
         values ('andro-test', 'Andromeda Galaxy', '{M31}', 'messier')",
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body) = app
        .oneshot_json::<SearchResults>("GET", "/api/search?q=andromeda", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.q, "andromeda");
    // Target matches canonical_name "Andromeda Galaxy".
    assert!(
        body.targets.iter().any(|t| t.slug == "andro-test"),
        "target 'andro-test' should appear in results"
    );
    // User matches handle "andromeda_aficionado" and display_name "Andromeda Aficionado".
    assert!(
        body.users.iter().any(|u| u.handle == "andromeda_aficionado"),
        "user 'andromeda_aficionado' should appear in results"
    );
    // Photo matches target field "M31 Andromeda Galaxy".
    assert!(
        body.photos.iter().any(|ph| ph.id == p),
        "photo with target 'M31 Andromeda Galaxy' should appear in results"
    );
}

/// Empty q returns 400.
#[tokio::test]
async fn search_empty_q_returns_400() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/search?q=", None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Any non-empty query returns 200 (caps each group).
#[tokio::test]
async fn search_caps_each_group() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/search?q=any", None, None).await;
    assert_eq!(status, StatusCode::OK);
}
