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

/// Integration seconds prefer the XISF-decoded total (`integration_s`),
/// fall back to per-sub exposure × sub count, and a photo with several
/// targets must not multiply the sums (the old LEFT JOIN fanout bug).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn public_profile_stats_integration_and_no_target_fanout() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;

    // Photo 1: XISF-style — only the decoded total is present.
    let p1 = app
        .ready_photo_with(uid, "AAAA0001", Some("NGC 5982"))
        .await;
    sqlx::query!(
        "update photos set integration_s = 401400, appreciations_count = 3 where id = $1",
        p1
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Photo 2: EXIF-style — per-sub exposure × sub count fallback.
    let p2 = app.ready_photo_with(uid, "BBBB0002", Some("M31")).await;
    sqlx::query!(
        "update photos set exposure_s = 300, sessions = 10, appreciations_count = 2 where id = $1",
        p2
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Give photo 1 TWO targets — sums must not double.
    for slug in ["ngc-5982", "ngc-5985"] {
        let tid = sqlx::query_scalar!(
            "insert into targets (slug, canonical_name, kind) values ($1, $2, 'ngc') returning id",
            slug,
            slug
        )
        .fetch_one(&app.pool)
        .await
        .unwrap();
        sqlx::query!(
            "insert into photo_targets (photo_id, target_id, source, is_primary) \
             values ($1, $2, 'manual', $3)",
            p1,
            tid,
            slug == "ngc-5982"
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }

    let (status, body) = app
        .oneshot_json::<PublicProfile>("GET", "/api/users/by-handle/pascal/profile", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.stats.frames, 2);
    // 401400 (decoded total) + 300 × 10 (fallback) — counted exactly once
    // each despite p1 having two targets.
    assert_eq!(body.stats.integration_seconds, 404_400);
    assert_eq!(body.stats.appreciations, 5);
    assert_eq!(body.stats.targets, 2);
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
