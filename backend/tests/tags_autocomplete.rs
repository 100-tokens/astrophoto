//! Integration tests for Task 35: GET /api/tags/autocomplete?q=
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used)]
fn config_for(url: &str) -> Config {
    Config {
        bind: "127.0.0.1:0".into(),
        log: "info".into(),
        database_url: url.into(),
        session_domain: "localhost".into(),
        session_secure: false,
        public_base_url: "http://localhost:8080".into(),
        s3_endpoint: None,
        s3_region: "us-east-1".into(),
        s3_bucket: "x".into(),
        s3_access_key: "a".into(),
        s3_secret_key: "s".into(),
        s3_path_style: true,
        cdn_base_url: "http://localhost:0/cdn".into(),
        cors_origin: None,
        oauth_google_client_id: String::new(),
        oauth_google_client_secret: String::new(),
        oauth_google_redirect_url: String::new(),
        smtp_host: "unused-in-tests".into(),
        smtp_port: 1025,
        smtp_user: String::new(),
        smtp_pass: String::new(),
        mail_from: "test <test@astrophoto.local>".into(),
        smtp_tls: false,
    }
}

async fn make_app() -> (axum::Router, sqlx::PgPool) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    // Keep `pg` alive for the duration of the test by leaking it.
    std::mem::forget(pg);
    let router = http::router(pool.clone(), config_for(&url), storage, Arc::new(mailer));
    (router, pool)
}

async fn get_json(app: axum::Router, uri: &str) -> serde_json::Value {
    let resp = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "expected 200 for {uri}");
    let bytes = axum::body::to_bytes(resp.into_body(), 65_536)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// ?q=alpha matches h-alpha by slug ILIKE.
#[tokio::test]
async fn tags_autocomplete_finds_by_slug_fragment() {
    let (app, pool) = make_app().await;
    // Insert 3 tags directly.
    sqlx::query!(
        "insert into tags (slug, name) values ($1, $2), ($3, $4), ($5, $6)",
        "narrowband",
        "narrowband",
        "widefield",
        "widefield",
        "h-alpha",
        "h-alpha",
    )
    .execute(&pool)
    .await
    .unwrap();

    let v = get_json(app, "/api/tags/autocomplete?q=alpha").await;
    let items = v["tags"].as_array().unwrap();
    assert_eq!(items.len(), 1, "expected exactly 1 result, got: {items:?}");
    assert_eq!(items[0]["slug"], "h-alpha");
    assert_eq!(items[0]["name"], "h-alpha");
}

/// ?q= (empty string) returns empty tags array without hitting the DB.
#[tokio::test]
async fn tags_autocomplete_empty_q_returns_empty() {
    let (app, _pool) = make_app().await;
    let v = get_json(app, "/api/tags/autocomplete?q=").await;
    let items = v["tags"].as_array().unwrap();
    assert!(items.is_empty(), "expected empty array for empty q");
}

/// ?q=narrow matches narrowband.
#[tokio::test]
async fn tags_autocomplete_finds_narrowband() {
    let (app, pool) = make_app().await;
    sqlx::query!(
        "insert into tags (slug, name) values ($1, $2), ($3, $4), ($5, $6)",
        "narrowband",
        "narrowband",
        "widefield",
        "widefield",
        "h-alpha",
        "h-alpha",
    )
    .execute(&pool)
    .await
    .unwrap();

    let v = get_json(app, "/api/tags/autocomplete?q=narrow").await;
    let items = v["tags"].as_array().unwrap();
    assert_eq!(items.len(), 1, "expected exactly 1 result, got: {items:?}");
    assert_eq!(items[0]["slug"], "narrowband");
}
