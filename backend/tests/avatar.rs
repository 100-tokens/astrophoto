//! Integration tests for the avatar upload flow.
//!
//! Uses `MemoryStorage` in lieu of MinIO/S3 — no S3 container needed. The
//! router is built inline (holding the `Arc<dyn Storage>`) so the test can
//! seed the temp object that the browser would otherwise PUT directly to S3.

mod common;

use std::sync::Arc;

use astrophoto::storage::MemoryStorage;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use bytes::Bytes;
use serde_json::{Value, json};
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn boot() -> (
    axum::Router,
    sqlx::PgPool,
    Arc<dyn astrophoto::storage::Storage>,
) {
    let pg = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
        .expect("postgres start");
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    // Leak the container so it lives until process exit (same trick as the
    // shared harness — the handle would otherwise drop and stop the DB).
    std::mem::forget(pg);
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = astrophoto::http::router(
        pool.clone(),
        common::config_for(&url),
        storage.clone(),
        Arc::new(mailer),
        None,
    );
    (app, pool, storage)
}

#[allow(clippy::unwrap_used)]
async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
    body: Option<Value>,
) -> (StatusCode, Vec<u8>) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie);
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1_048_576)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

/// Drive the full flow: init → (simulate browser PUT) → finalize.
#[allow(clippy::unwrap_used)]
async fn upload_avatar(
    app: &axum::Router,
    storage: &Arc<dyn astrophoto::storage::Storage>,
    cookie: &str,
    bytes: &[u8],
) -> String {
    let (status, body) = send(
        app,
        "POST",
        "/api/me/avatar/init",
        cookie,
        Some(json!({ "size": bytes.len(), "mime": "image/jpeg" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "init: {}",
        String::from_utf8_lossy(&body)
    );
    let init: Value = serde_json::from_slice(&body).unwrap();
    let avatar_id = init["avatar_id"].as_str().unwrap().to_string();
    assert!(init["presigned_put_url"].as_str().is_some());

    // Simulate the browser's direct-to-S3 PUT of the raw image.
    storage
        .put(
            &format!("avatar-uploads/{avatar_id}"),
            "image/jpeg",
            Bytes::copy_from_slice(bytes),
        )
        .await
        .unwrap();

    let (status, body) = send(
        app,
        "POST",
        "/api/me/avatar/finalize",
        cookie,
        Some(json!({ "avatar_id": avatar_id })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "finalize: {}",
        String::from_utf8_lossy(&body)
    );
    avatar_id
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn upload_sets_avatar_and_writes_cdn_display_master() {
    let (app, pool, storage) = boot().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice").await;
    let user_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let sample: &[u8] = include_bytes!("fixtures/sample.jpg");

    let avatar_id = upload_avatar(&app, &storage, &cookie, sample).await;

    // DB pointer flipped to the new avatar.
    let stored: Option<Uuid> =
        sqlx::query_scalar!("select avatar_id from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.map(|u| u.to_string()), Some(avatar_id.clone()));

    // The CDN-served display master exists at display/<avatar_id>.jpg and is
    // a real JPEG (SOI marker) — this is exactly what `/img/<avatar_id>` →
    // Lambda@Edge fetches and transforms.
    let master = storage
        .get(&format!("display/{avatar_id}.jpg"))
        .await
        .unwrap()
        .expect("display master must exist");
    assert_eq!(&master[..2], &[0xFF, 0xD8], "display master must be a JPEG");

    // The temp upload object is cleaned up.
    assert!(
        storage
            .get(&format!("avatar-uploads/{avatar_id}"))
            .await
            .unwrap()
            .is_none(),
        "temp upload object should be deleted after finalize"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reupload_replaces_and_garbage_collects_old_master() {
    let (app, pool, storage) = boot().await;
    let cookie = common::signup_and_cookie(&app, &pool, "bob@example.com", "bob").await;
    let sample: &[u8] = include_bytes!("fixtures/sample.jpg");

    let first = upload_avatar(&app, &storage, &cookie, sample).await;
    let second = upload_avatar(&app, &storage, &cookie, sample).await;
    assert_ne!(first, second, "each upload mints a fresh avatar id");

    assert!(
        storage
            .get(&format!("display/{first}.jpg"))
            .await
            .unwrap()
            .is_none(),
        "previous display master should be GC'd on re-upload"
    );
    assert!(
        storage
            .get(&format!("display/{second}.jpg"))
            .await
            .unwrap()
            .is_some(),
        "new display master should be live"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn clear_removes_avatar_and_object() {
    let (app, pool, storage) = boot().await;
    let cookie = common::signup_and_cookie(&app, &pool, "carol@example.com", "carol").await;
    let user_id = common::lookup_user_id(&pool, "carol@example.com").await;
    let sample: &[u8] = include_bytes!("fixtures/sample.jpg");

    let avatar_id = upload_avatar(&app, &storage, &cookie, sample).await;

    let (status, _) = send(&app, "DELETE", "/api/me/avatar", &cookie, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let stored: Option<Uuid> =
        sqlx::query_scalar!("select avatar_id from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, None, "avatar_id cleared");
    assert!(
        storage
            .get(&format!("display/{avatar_id}.jpg"))
            .await
            .unwrap()
            .is_none(),
        "display master deleted on clear"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn init_rejects_bad_mime_and_oversize() {
    let (app, pool, _storage) = boot().await;
    let cookie = common::signup_and_cookie(&app, &pool, "dave@example.com", "dave").await;

    let (status, _) = send(
        &app,
        "POST",
        "/api/me/avatar/init",
        &cookie,
        Some(json!({ "size": 1024, "mime": "image/gif" })),
    )
    .await;
    assert!(status.is_client_error(), "gif rejected: got {status}");

    let (status, _) = send(
        &app,
        "POST",
        "/api/me/avatar/init",
        &cookie,
        Some(json!({ "size": 50 * 1024 * 1024, "mime": "image/jpeg" })),
    )
    .await;
    assert!(status.is_client_error(), "oversize rejected: got {status}");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn avatar_endpoints_require_auth() {
    let (app, _pool, _storage) = boot().await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/me/avatar/init",
        "session=bogus",
        Some(json!({ "size": 1024, "mime": "image/jpeg" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
