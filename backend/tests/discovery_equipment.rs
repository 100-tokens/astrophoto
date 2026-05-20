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
    // canonical_name uses spaces (the URL slug "asi2600mc-pro" is
    // converted via dash→space before lookup, see
    // discovery::equipment::canonical_for).
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model) \
         values ('camera', 'asi2600mc pro', 'ASI2600MC Pro', 1, '', 'ASI2600MC Pro')",
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Set the photo's camera field to match the canonical_name
    // case-insensitively — the paired-rail join uses lower(p.camera)
    // = ei.canonical_name, so it must NOT contain a dash.
    sqlx::query("update photos set camera = 'ASI2600MC PRO' where id = $1")
        .bind(photo_id)
        .execute(&app.pool)
        .await
        .unwrap();

    // Insert a second equipment item (a telescope) that co-occurs on the same photo
    // so the paired rail has something to show.
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model) \
         values ('telescope', 'at80ed', 'AT80ED', 1, '', 'AT80ED')",
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
    assert_eq!(body.equipment.canonical_name, "asi2600mc pro");
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

/// Same-brand sibling rail: same kind, shared leading token in
/// canonical_name. Sorted by usage_count desc.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_page_returns_brand_siblings() {
    let app = TestApp::launch().await;
    // Three Antlia filters; one unrelated brand (Astrodon) to confirm
    // the brand filter actually filters. Different usage_count so we
    // can assert ordering.
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model) values
            ('filter', 'antlia 3nm h alpha pro', 'Antlia 3nm H-alpha Pro', 1, 'Antlia',  '3nm H-alpha Pro'),
            ('filter', 'antlia 3nm oiii pro',    'Antlia 3nm OIII Pro',    5, 'Antlia',  '3nm OIII Pro'),
            ('filter', 'antlia 3nm sii pro',     'Antlia 3nm SII Pro',     3, 'Antlia',  '3nm SII Pro'),
            ('filter', 'astrodon ha 5nm',        'Astrodon Hα 5nm',        9, 'Astrodon','Hα 5nm')",
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body) = app
        .oneshot_json::<EquipmentPage>(
            "GET",
            "/api/equipment/filter/antlia-3nm-h-alpha-pro",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.siblings.len(),
        2,
        "two other Antlia filters, no Astrodon"
    );
    // usage_count desc → OIII (5) before SII (3).
    assert_eq!(body.siblings[0].slug, "antlia-3nm-oiii-pro");
    assert_eq!(body.siblings[0].usage_count, 5);
    assert_eq!(body.siblings[1].slug, "antlia-3nm-sii-pro");
    assert_eq!(body.siblings[1].usage_count, 3);
}

/// Single-token canonical_name (no whitespace) has no brand prefix
/// to extract — siblings is empty even when other same-kind items
/// exist. Uses a mount canonical without a space; the multi-word
/// Sky-Watcher EQ6-R Pro is intentionally NOT considered a sibling
/// because the single-token item has no brand to share.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_page_no_siblings_for_brandless_name() {
    let app = TestApp::launch().await;
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model) values
            ('mount', 'cem40',                  'CEM40',                  1, '',            'CEM40'),
            ('mount', 'sky-watcher eq6-r pro',  'Sky-Watcher EQ6-R Pro',  9, 'Sky-Watcher', 'EQ6-R Pro')",
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body) = app
        .oneshot_json::<EquipmentPage>("GET", "/api/equipment/mount/cem40", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.siblings.is_empty(),
        "single-token canonical_name has no brand prefix"
    );
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
