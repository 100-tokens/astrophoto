mod common;

use astrophoto::api_types::PublicProfile;
use axum::http::StatusCode;
use chrono::Datelike;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn public_profile_returns_full_shape() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Marie", "marie", "marie@x.test")
        .await;

    // Populate profile fields.
    app.oneshot(
        "PATCH",
        "/api/me/profile",
        Some(&cookie),
        Some(serde_json::json!({
            "tagline": "Hunting deep-sky from a Bortle 6 backyard",
            "bio_html": "<p>Hi</p>",
            "equipment": { "telescope": "RedCat 51", "camera": "ASI2600MC" },
            "location": { "location_text": "Lyon, FR", "bortle_class": 6, "sqm": 19.8 },
            "social_links": [{ "platform": "twitter", "url": "https://twitter.com/marie" }]
        })),
    )
    .await;

    let p1 = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", Some("M42")).await;

    app.oneshot(
        "POST",
        &format!("/api/me/featured/{p1}"),
        Some(&cookie),
        None,
    )
    .await;
    app.oneshot(
        "POST",
        &format!("/api/me/featured/{p2}"),
        Some(&cookie),
        None,
    )
    .await;
    app.oneshot(
        "POST",
        "/api/me/cover",
        Some(&cookie),
        Some(serde_json::json!({ "photo_id": p1 })),
    )
    .await;

    let (status, body) = app
        .oneshot_json::<PublicProfile>("GET", "/api/users/by-handle/marie/profile", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.handle, "marie");
    assert_eq!(body.display_name, "Marie");
    assert_eq!(
        body.tagline.as_deref(),
        Some("Hunting deep-sky from a Bortle 6 backyard")
    );
    assert_eq!(body.equipment.telescope.as_deref(), Some("RedCat 51"));
    assert_eq!(body.location.bortle_class, Some(6));
    assert_eq!(body.social_links.len(), 1);
    assert_eq!(body.featured.len(), 2);
    assert_eq!(body.featured[0].featured_position, 1);
    assert_eq!(body.featured[0].id, p1);
    assert_eq!(body.featured[1].id, p2);
    assert_eq!(body.cover.as_ref().unwrap().id, p1);
    assert_eq!(body.stats.frames, 2);
    let current_year = chrono::Utc::now().date_naive().year();
    assert_eq!(body.stats.member_since_year, current_year);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn public_profile_404_for_unknown_handle() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/users/by-handle/nobody/profile", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
