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
