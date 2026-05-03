mod common;

use astrophoto::api_types::{Profile, SocialLink};
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn get_profile_returns_full_shape_for_fresh_user() {
    let app = TestApp::launch().await;
    let (cookie, _user_id) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let (status, body) = app
        .oneshot_json::<Profile>("GET", "/api/me/profile", Some(&cookie), None)
        .await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body.display_name, "Marie");
    assert!(body.tagline.is_none());
    assert!(body.bio_html.is_none());
    assert!(body.cover_photo_id.is_none());
    assert!(body.equipment.telescope.is_none());
    assert!(body.equipment.camera.is_none());
    assert!(body.equipment.mount.is_none());
    assert!(body.equipment.filters.is_none());
    assert!(body.equipment.guiding.is_none());
    assert!(body.location.location_text.is_none());
    assert!(body.location.bortle_class.is_none());
    assert!(body.location.sqm.is_none());
    assert_eq!(body.social_links, Vec::<SocialLink>::new());
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn patch_profile_writes_tagline_and_bio_with_sanitisation() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let body = serde_json::json!({
        "tagline": "Hunting deep-sky from a Bortle 6 backyard",
        "bio_html": "<p>Hi <strong>world</strong></p><script>alert(1)</script>"
    });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/profile", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_status, body) = app
        .oneshot_json::<Profile>("GET", "/api/me/profile", Some(&cookie), None)
        .await;
    assert_eq!(body.tagline.as_deref(), Some("Hunting deep-sky from a Bortle 6 backyard"));
    let bio = body.bio_html.unwrap();
    assert!(bio.contains("<strong>"));
    assert!(!bio.contains("<script>"), "script must be stripped: {bio}");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn patch_profile_writes_equipment_and_location() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let body = serde_json::json!({
        "equipment": {
            "telescope": "RedCat 51",
            "camera": "ZWO ASI2600MC",
            "mount": "ZWO AM5",
            "filters": "Optolong L-Pro",
            "guiding": "ASI120MM"
        },
        "location": {
            "location_text": "Lyon, FR",
            "bortle_class": 6,
            "sqm": 19.8
        }
    });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/profile", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_status, body) = app
        .oneshot_json::<Profile>("GET", "/api/me/profile", Some(&cookie), None)
        .await;
    assert_eq!(body.equipment.telescope.as_deref(), Some("RedCat 51"));
    assert_eq!(body.equipment.camera.as_deref(), Some("ZWO ASI2600MC"));
    assert_eq!(body.location.location_text.as_deref(), Some("Lyon, FR"));
    assert_eq!(body.location.bortle_class, Some(6));
    assert!((body.location.sqm.unwrap() - 19.8).abs() < 0.01);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn patch_profile_validates_social_links() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    // Wrong host for the named platform → 400.
    let bad = serde_json::json!({
        "social_links": [{ "platform": "twitter", "url": "https://evil.example/marie" }]
    });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/profile", Some(&cookie), Some(bad))
        .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);

    // Correct mapping → 204 + persisted.
    let ok = serde_json::json!({
        "social_links": [
            { "platform": "twitter",   "url": "https://twitter.com/marie" },
            { "platform": "instagram", "url": "https://instagram.com/marie" }
        ]
    });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/profile", Some(&cookie), Some(ok))
        .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_status, body) = app
        .oneshot_json::<Profile>("GET", "/api/me/profile", Some(&cookie), None)
        .await;
    assert_eq!(body.social_links.len(), 2);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn patch_profile_clears_field_when_explicit_null() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    // Set tagline.
    app.oneshot(
        "PATCH",
        "/api/me/profile",
        Some(&cookie),
        Some(serde_json::json!({ "tagline": "first" })),
    )
    .await;

    // Clear with explicit null.
    let (status, _) = app
        .oneshot(
            "PATCH",
            "/api/me/profile",
            Some(&cookie),
            Some(serde_json::json!({ "tagline": null })),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_status, body) = app
        .oneshot_json::<Profile>("GET", "/api/me/profile", Some(&cookie), None)
        .await;
    assert!(body.tagline.is_none());
}
