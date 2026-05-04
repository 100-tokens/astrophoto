mod common;

use astrophoto::api_types::EquipmentPage;
use axum::http::StatusCode;
use common::TestApp;

/// Happy-path: equipment page returns meta + photos + paired rail.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_page_returns_meta_photos_and_paired_rail() {
    let app = TestApp::launch().await;
    let (_, uid) = app
        .signup_with_handle("Alice", "alice", "alice@x.test")
        .await;
    let photo_id = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;

    // Insert the equipment_items row for our camera.
    // canonical_name is lowercased; no spaces so it's URL-safe as a path segment.
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) \
         values ('camera', 'asi2600mc-pro', 'ASI2600MC Pro', 1)",
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Set the photo's camera field to match (case-insensitive).
    sqlx::query("update photos set camera = 'ASI2600MC-PRO' where id = $1")
        .bind(photo_id)
        .execute(&app.pool)
        .await
        .unwrap();

    // Insert a second equipment item (a telescope) that co-occurs on the same photo
    // so the paired rail has something to show.
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) \
         values ('telescope', 'at80ed', 'AT80ED', 1)",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query("update photos set scope = 'AT80ED' where id = $1")
        .bind(photo_id)
        .execute(&app.pool)
        .await
        .unwrap();

    let (status, body) = app
        .oneshot_json::<EquipmentPage>("GET", "/api/equipment/camera/asi2600mc-pro", None, None)
        .await;
    assert_eq!(status, StatusCode::OK, "should return 200");
    assert_eq!(body.equipment.kind, "camera");
    assert_eq!(body.equipment.canonical_name, "asi2600mc-pro");
    assert_eq!(body.equipment.display_name, "ASI2600MC Pro");
    assert_eq!(body.equipment.photo_count, 1, "one photo uses this camera");
    assert_eq!(body.page.photos.len(), 1, "one photo in the grid");
    assert_eq!(body.page.photos[0].id, photo_id);
    // Paired rail should contain the co-occurring telescope.
    assert_eq!(
        body.paired.len(),
        1,
        "telescope co-occurs on the same photo"
    );
    assert_eq!(body.paired[0].kind, "telescope");
    assert_eq!(body.paired[0].slug, "at80ed");
}

/// 404 for an unknown kind value (not in the whitelist).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_404_for_unknown_kind() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/equipment/foo/bar", None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// 404 for a valid kind but a slug that doesn't exist in equipment_items.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_404_for_unknown_slug() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot(
            "GET",
            "/api/equipment/camera/nonexistent-camera",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
