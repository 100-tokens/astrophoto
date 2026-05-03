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
