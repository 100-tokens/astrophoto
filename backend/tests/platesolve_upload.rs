//! Integration tests for `POST /api/photos/:id/platesolve`.
//!
//! We cover the synchronous handler path (route mounting, magic-byte
//! gate, owner check, sentinel conflict) but NOT the spawned solve
//! body — that calls the upstream service and needs a mock HTTP
//! server. The client's error mapping is covered by unit tests in
//! `src/photos/platesolve.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::photos::platesolve::{PlatesolveClient, SOLVING_SENTINEL};
use astrophoto::storage::MemoryStorage;
use astrophoto::{Config, db, http, mail::Mailer};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use bytes::Bytes;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

mod common;

fn config_with_platesolve(url: &str, base: Option<&str>, key: Option<&str>) -> Config {
    let mut cfg = common::config_for(url);
    cfg.platesolve_base_url = base.map(str::to_string);
    cfg.platesolve_api_key = key.map(str::to_string);
    cfg
}

async fn launch(with_platesolve: bool) -> (Router, sqlx::PgPool) {
    let pg = PgImage::default()
        .with_tag("16-alpine")
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = Mailer::for_test();
    let (cfg, platesolve) = if with_platesolve {
        // Unreachable URL is fine — these tests never let the spawned
        // task actually call the upstream (handler returns 4xx before
        // spawn, or we don't drain the spawned task).
        let cfg = config_with_platesolve(&url, Some("http://127.0.0.1:1"), Some("test-key"));
        let client = PlatesolveClient::from_config(&cfg)
            .expect("client builds")
            .map(Arc::new);
        (cfg, client)
    } else {
        (common::config_for(&url), None)
    };
    // Leak the container so it lives for the test process.
    std::mem::forget(pg);
    let app = http::router(
        pool.clone(),
        cfg,
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        platesolve,
    );
    (app, pool)
}

async fn signup_and_cookie(app: &Router, pool: &sqlx::PgPool, email: &str, handle: &str) -> String {
    common::signup_and_cookie(app, pool, email, handle).await
}

async fn insert_photo(pool: &sqlx::PgPool, owner: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    // Runtime query (not `sqlx::query!`) so the test compiles without
    // requiring a freshly-prepared `.sqlx/` cache entry for the new
    // exact-SQL string. The existing schema validates the shape at
    // runtime; deferred type-checking is fine in test setup.
    sqlx::query(
        r#"insert into photos
              (id, owner_id, storage_key, original_name, bytes, mime, status,
               short_id, original_uploaded_at)
            values ($1, $2, 'k', 'o.jpg', 1, 'image/jpeg', 'ready', $3, now())"#,
    )
    .bind(id)
    .bind(owner)
    .bind(Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn user_id(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("select id from users where email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn read_platesolve_error(pool: &sqlx::PgPool, photo_id: Uuid) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>("select platesolve_error from photos where id = $1")
        .bind(photo_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn multipart_body(boundary: &str, file_bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"upload.xisf\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/x-xisf\r\n\r\n");
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

async fn post_platesolve(
    app: &Router,
    photo_id: Uuid,
    cookie: &str,
    body_bytes: &[u8],
    boundary: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/platesolve"))
                .header(header::COOKIE, cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body_bytes.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn route_not_mounted_without_platesolve_client() {
    let (app, pool) = launch(false).await;
    let cookie = signup_and_cookie(&app, &pool, "alice@example.com", "alice").await;
    let user = user_id(&pool, "alice@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    let boundary = "boundary-not-mounted";
    let body = multipart_body(boundary, b"XISF0100");
    let resp = post_platesolve(&app, photo_id, &cookie, &body, boundary).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "route must 404 when APP_PLATESOLVE_BASE_URL is unset"
    );
}

#[tokio::test]
async fn rejects_non_xisf_body() {
    let (app, pool) = launch(true).await;
    let cookie = signup_and_cookie(&app, &pool, "bob@example.com", "bob").await;
    let user = user_id(&pool, "bob@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    let boundary = "boundary-bad-magic";
    // Random bytes — won't pass the XISF sniff.
    let body = multipart_body(boundary, b"NOT_AN_XISF_FILE_AT_ALL");
    let resp = post_platesolve(&app, photo_id, &cookie, &body, boundary).await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "non-XISF body must be rejected at the magic-byte gate (400)"
    );

    // Sentinel must NOT have been set — the gate rejects before claim.
    let err = read_platesolve_error(&pool, photo_id).await;
    assert!(err.is_none(), "platesolve_error must remain null on 400");
}

#[tokio::test]
async fn cross_owner_returns_404() {
    let (app, pool) = launch(true).await;
    let _alice = signup_and_cookie(&app, &pool, "alice2@example.com", "alice2").await;
    let bob = signup_and_cookie(&app, &pool, "bob2@example.com", "bob2").await;
    let alice = user_id(&pool, "alice2@example.com").await;
    let photo_id = insert_photo(&pool, alice).await;

    let boundary = "boundary-cross-owner";
    let body = multipart_body(boundary, b"XISF0100\x00\x00\x00\x00");
    let resp = post_platesolve(&app, photo_id, &bob, &body, boundary).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "cross-owner POST must 404 (leak prevention)"
    );
}

#[tokio::test]
async fn conflict_when_already_solving() {
    let (app, pool) = launch(true).await;
    let cookie = signup_and_cookie(&app, &pool, "carol@example.com", "carol").await;
    let user = user_id(&pool, "carol@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    // Pretend a previous request already claimed the sentinel.
    sqlx::query(r#"update photos set platesolve_error = $1 where id = $2"#)
        .bind(SOLVING_SENTINEL)
        .bind(photo_id)
        .execute(&pool)
        .await
        .unwrap();

    let boundary = "boundary-already-solving";
    let body = multipart_body(boundary, b"XISF0100\x00\x00\x00\x00");
    let resp = post_platesolve(&app, photo_id, &cookie, &body, boundary).await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "second concurrent solve must 409"
    );

    // Read the body to confirm the AppError code.
    let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["error"], "conflict");
}

// ─────────────────────────────────────────────────────── /platesolve-status

async fn get_status(app: &Router, photo_id: Uuid, cookie: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/photos/{photo_id}/platesolve-status"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, v)
}

#[tokio::test]
async fn status_idle_when_never_solved() {
    // Mount without the upstream client to confirm the status route
    // is independent of the POST route's conditional mount.
    let (app, pool) = launch(false).await;
    let cookie = signup_and_cookie(&app, &pool, "dave@example.com", "dave").await;
    let user = user_id(&pool, "dave@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    let (status, body) = get_status(&app, photo_id, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "idle");
    assert!(body["error"].is_null());
    assert!(body["solvedAt"].is_null());
}

#[tokio::test]
async fn status_solving_when_sentinel_set() {
    let (app, pool) = launch(false).await;
    let cookie = signup_and_cookie(&app, &pool, "eve@example.com", "everyone").await;
    let user = user_id(&pool, "eve@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    sqlx::query(r#"update photos set platesolve_error = $1 where id = $2"#)
        .bind(SOLVING_SENTINEL)
        .bind(photo_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = get_status(&app, photo_id, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "solving");
    assert!(
        body["error"].is_null(),
        "in-progress state hides the sentinel"
    );
}

#[tokio::test]
async fn status_solved_when_solved_at_set() {
    let (app, pool) = launch(false).await;
    let cookie = signup_and_cookie(&app, &pool, "frank@example.com", "frank").await;
    let user = user_id(&pool, "frank@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    sqlx::query(
        r#"update photos
              set ra_deg = $1,
                  dec_deg = $2,
                  platesolve_pixel_scale_arcsec = $3,
                  platesolve_rms_arcsec = $4,
                  platesolve_matched_count = $5,
                  platesolve_detected_count = $6,
                  platesolve_solved_at = now()
            where id = $7"#,
    )
    .bind(202.4697_f64)
    .bind(47.1953_f64)
    .bind(0.524_f32)
    .bind(0.017_f32)
    .bind(45_i32)
    .bind(4297_i32)
    .bind(photo_id)
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get_status(&app, photo_id, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "solved");
    assert!(body["solvedAt"].is_string());
    assert_eq!(body["raDeg"].as_f64().unwrap(), 202.4697);
    assert_eq!(body["matchedCount"].as_i64().unwrap(), 45);
    assert_eq!(body["detectedCount"].as_i64().unwrap(), 4297);
}

#[tokio::test]
async fn status_failed_when_error_set() {
    let (app, pool) = launch(false).await;
    let cookie = signup_and_cookie(&app, &pool, "grace@example.com", "grace").await;
    let user = user_id(&pool, "grace@example.com").await;
    let photo_id = insert_photo(&pool, user).await;

    sqlx::query(r#"update photos set platesolve_error = $1 where id = $2"#)
        .bind("solve failed: too few stars")
        .bind(photo_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = get_status(&app, photo_id, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "failed");
    assert_eq!(body["error"], "solve failed: too few stars");
}

#[tokio::test]
async fn status_cross_owner_returns_404() {
    let (app, pool) = launch(false).await;
    let _alice = signup_and_cookie(&app, &pool, "alice3@example.com", "alice3").await;
    let bob = signup_and_cookie(&app, &pool, "bob3@example.com", "bob3").await;
    let alice = user_id(&pool, "alice3@example.com").await;
    // insert_photo leaves the photo unpublished (draft) — a non-owner must
    // not see a draft's solve telemetry.
    let photo_id = insert_photo(&pool, alice).await;

    let (status, _body) = get_status(&app, photo_id, &bob).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn status_public_for_published_photo() {
    // A PUBLISHED photo exposes its solve telemetry to anyone — the public
    // photo page's celestial overlay needs the WCS to project markers.
    let (app, pool) = launch(false).await;
    let _alice = signup_and_cookie(&app, &pool, "alice4@example.com", "alice4").await;
    let alice = user_id(&pool, "alice4@example.com").await;
    let photo_id = insert_photo(&pool, alice).await;
    sqlx::query("update photos set published_at = now() where id = $1")
        .bind(photo_id)
        .execute(&pool)
        .await
        .unwrap();

    // No cookie at all → anonymous visitor still gets 200.
    let (status, body) = get_status(&app, photo_id, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "idle");
}

// ──────────────────────────────────────────── auto-calibrate (Phase 3)

#[tokio::test]
async fn auto_calibrate_marks_photo_failed_when_upstream_unreachable() {
    // Exercises the primary-XISF-upload auto-trigger end-to-end with
    // an unreachable plate-solve URL. The spawned task fetches the
    // XISF from storage, calls run_solve (which fails with Transport
    // after 3 attempts × 1s/2s/4s backoff), then marks the photo
    // `failed` with the error in `pipeline_error`. Polling-with-
    // timeout because the spawned task is fire-and-forget.
    use std::time::{Duration, Instant};

    let pg = PgImage::default()
        .with_tag("16-alpine")
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = Mailer::for_test();

    let mut cfg = config_with_platesolve(&url, Some("http://127.0.0.1:1"), Some("test-key"));
    cfg.platesolve_timeout_secs = 2; // tight so the test finishes quickly
    let client = PlatesolveClient::from_config(&cfg)
        .expect("client builds")
        .map(Arc::new);
    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let permits = Arc::new(tokio::sync::Semaphore::new(1));
    // We need AppState directly (auto_calibrate_xisf takes it),
    // bypassing the http::router builder.
    let state = astrophoto::http::AppState {
        pool: pool.clone(),
        config: Arc::new(cfg),
        storage: Arc::clone(&storage),
        mailer: Arc::new(mailer),
        platesolve: client,
        platesolve_permits: permits,
    };

    // Build a photo in `awaiting-calibration` state with a real
    // storage key holding a minimal XISF body.
    let owner = sqlx::query_scalar::<_, Uuid>(
        r#"insert into users (email, password_hash, display_name, handle)
            values ('autocal@example.com',
                    '$argon2id$v=19$m=19456,t=2,p=1$AAAA$AAAA',
                    'Auto', 'autocal') returning id"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    sqlx::query(
        r#"insert into photos
              (id, owner_id, storage_key, original_name, bytes, mime, status,
               short_id, original_uploaded_at)
            values ($1, $2, $3, 'master.xisf', 1024, 'application/x-xisf',
                    'awaiting-calibration', $4, now())"#,
    )
    .bind(photo_id)
    .bind(owner)
    .bind(&storage_key)
    .bind(Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .unwrap();
    // Seed storage with a minimal XISF body (signature only — the
    // upstream never gets to validate it because the connect fails).
    let mut xisf = b"XISF0100".to_vec();
    xisf.extend_from_slice(&[0u8; 16]);
    storage
        .put(&storage_key, "application/x-xisf", Bytes::from(xisf))
        .await
        .unwrap();

    // Fire the auto-trigger. Spawn returns immediately.
    astrophoto::photos::platesolve_upload::auto_calibrate_xisf(state, photo_id, storage_key, owner);

    // Poll for the terminal state. 3 attempts × (1s + 2s + 4s
    // backoff + 2s timeout) ≈ 21s worst case; pad to 30s.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut final_status: String = "(timeout)".into();
    let mut final_error: Option<String> = None;
    while Instant::now() < deadline {
        let row: (String, Option<String>) = sqlx::query_as::<_, (String, Option<String>)>(
            "select status, pipeline_error from photos where id = $1",
        )
        .bind(photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        if row.0 == "failed" || row.0 == "ready" {
            final_status = row.0;
            final_error = row.1;
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert_eq!(
        final_status, "failed",
        "auto-calibrate against unreachable URL should transition to failed (got `{final_status}`, error: {final_error:?})"
    );
    let err = final_error.unwrap_or_default();
    assert!(
        err.contains("network failure")
            || err.contains("connection")
            || err.contains("timeout")
            || err.contains("Transport"),
        "pipeline_error should mention a network/transport failure; got: {err}"
    );
}
