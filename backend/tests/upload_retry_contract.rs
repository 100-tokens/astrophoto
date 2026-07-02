//! Retry contract for failed uploads.
//!
//! A finalize-stage failure (decode error, magic mismatch, S3 hiccup)
//! leaves the row status='failed'. The upload page's Retry button first
//! DELETEs the stale row, then re-inits the same file — both legs used
//! to 409 (cancel refused failed rows; the owner+hash dedup matched
//! them), wedging the retry loop forever. The contract now: failed rows
//! are cancellable, and they don't count as "already uploaded".

mod common;

use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

/// Insert a finalize-failed row with a known hash.
#[allow(clippy::unwrap_used)]
async fn insert_failed_photo(app: &TestApp, owner: Uuid, hash: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!(
        "insert into photos
            (id, owner_id, storage_key, original_name, bytes, mime, status,
             last_step, short_id, original_hash, pipeline_error, original_uploaded_at)
         values ($1, $2, $3, 'frame.jpg', 1000, 'image/jpeg', 'failed',
                 'upload', $4, $5, 'decode failed', now())",
        id,
        owner,
        format!("originals/{id}"),
        &id.to_string()[..8],
        hash
    )
    .execute(&app.pool)
    .await
    .unwrap();
    id
}

fn init_body(hash: &str) -> serde_json::Value {
    serde_json::json!({
        "files": [{"name": "frame.jpg", "size": 1000, "mime": "image/jpeg", "hash": hash}]
    })
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn failed_row_is_cancellable() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;
    let photo_id = insert_failed_photo(&app, uid, "deadbeef01").await;

    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/uploads/{photo_id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "failed rows are cancellable"
    );

    let gone = sqlx::query_scalar!("select count(*) from photos where id = $1", photo_id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(gone, Some(0), "row deleted");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reinit_same_hash_succeeds_after_failure() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;
    insert_failed_photo(&app, uid, "deadbeef02").await;

    // Same bytes, fresh init: must NOT 409 on the failed row.
    let (status, body) = app
        .oneshot_json::<serde_json::Value>(
            "POST",
            "/api/uploads/init",
            Some(&cookie),
            Some(init_body("deadbeef02")),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "failed rows must not block re-upload: {body}"
    );
    assert!(body["files"][0]["presigned_put_url"].is_string());
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pending_row_still_dedups() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app
        .signup_with_handle("Pascal", "pascal", "pascal@x.test")
        .await;

    let (status, _) = app
        .oneshot_json::<serde_json::Value>(
            "POST",
            "/api/uploads/init",
            Some(&cookie),
            Some(init_body("deadbeef03")),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Second init of the same hash while the first row is pending → 409.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/uploads/init",
            Some(&cookie),
            Some(init_body("deadbeef03")),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "live rows still dedup");
}
