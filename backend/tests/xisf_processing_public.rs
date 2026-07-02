//! `GET /api/photos/:id/processing` — the public boundary must scrub
//! per-subframe filesystem paths out of processing tables and must not
//! serve draft photos' history to anonymous callers.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

/// A stored report shaped like a real WBPP master: one file-list table
/// (private local paths), one numeric weights table, one curve table.
fn stored_report() -> serde_json::Value {
    serde_json::json!({
        "creatorApp": "PixInsight 1.9.3",
        "creatorModule": null,
        "creatorOs": "macOS",
        "createdAt": null,
        "displayStretch": null,
        "whiteBalance": null,
        "observation": null,
        "totalDurationS": 1321.0,
        "pipeline": [{
            "position": 0,
            "className": "ImageIntegration",
            "label": "Image Integration",
            "category": "Stacking",
            "summary": "Stack calibrated sub-exposures",
            "version": "256",
            "enabled": true,
            "startedAt": null,
            "durationS": 1201.0,
            "params": [
                {"key": "inputHints", "value": "fits-keywords normalize", "truncated": false}
            ],
            "tables": [
                {
                    "id": "images",
                    "kind": "generic",
                    "columns": ["enabled", "path"],
                    "rows": [
                        ["true", "/Volumes/Pascal4Tb/astrophotos/NGC5982/Light_300s_0001.xisf"],
                        ["true", "/Volumes/Pascal4Tb/astrophotos/NGC5982/Light_300s_0002.xisf"]
                    ]
                },
                {
                    "id": "weights",
                    "kind": "generic",
                    "columns": ["weightRK"],
                    "rows": [["0.39693"], ["0.44206"]]
                },
                {
                    "id": "K",
                    "kind": "curve",
                    "columns": ["x", "y"],
                    "rows": [["0.0", "0.0"], ["1.0", "1.0"]]
                }
            ]
        }]
    })
}

#[allow(clippy::unwrap_used)]
async fn set_processing_json(app: &TestApp, photo_id: Uuid) {
    sqlx::query!(
        "update photos set processing_json = $1 where id = $2",
        stored_report(),
        photo_id
    )
    .execute(&app.pool)
    .await
    .unwrap();
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn public_processing_scrubs_paths_keeps_charts() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;
    let photo_id = app.ready_photo(uid).await;
    set_processing_json(&app, photo_id).await;

    let (status, body) = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/photos/{photo_id}/processing"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // No local path anywhere in the payload.
    assert!(
        !body.to_string().contains("/Volumes/"),
        "private paths must not reach the public payload: {body}"
    );

    let tables = body["pipeline"][0]["tables"].as_array().unwrap();
    // File-list table emptied, numeric tables intact.
    assert_eq!(tables[0]["rows"].as_array().unwrap().len(), 0);
    assert_eq!(tables[1]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(tables[2]["rows"].as_array().unwrap().len(), 2);
    // Step metadata still tells the story without the paths.
    assert_eq!(body["pipeline"][0]["label"], "Image Integration");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn draft_processing_hidden_from_anonymous_visible_to_owner() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;
    // Draft: no published_at.
    let photo_id = common::insert_stub_photo(&app.pool, uid, None, None, None).await;
    set_processing_json(&app, photo_id).await;

    // Anonymous caller gets the same null body as "no report" — draft
    // existence is not probeable by UUID.
    let (status, body) = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/photos/{photo_id}/processing"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_null(), "draft report must be null for anon: {body}");

    // The owner still sees it.
    let (status, body) = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/photos/{photo_id}/processing"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["creatorApp"], "PixInsight 1.9.3");
}
