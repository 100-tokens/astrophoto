//! Integration tests for Task 34: GET /api/targets/autocomplete?q=
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
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
        cdn_local_fallback: false,
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

async fn make_app() -> axum::Router {
    let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    // Keep `pg` alive for the duration of the test by leaking it.
    // testcontainers drops the container when the handle is dropped.
    std::mem::forget(pg);
    http::router(pool, config_for(&url), storage, Arc::new(mailer))
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

/// ?q=Andromeda matches m31 via canonical_name ILIKE.
#[tokio::test]
async fn targets_autocomplete_finds_messier_by_alias() {
    let app = make_app().await;
    let v = get_json(app, "/api/targets/autocomplete?q=Andromeda").await;
    let items = v["targets"].as_array().unwrap();
    assert!(
        items.iter().any(|t| t["slug"] == "m31"),
        "expected m31 in results, got: {items:?}"
    );
}

/// ?q=m31 matches slug exactly and returns exactly m31.
#[tokio::test]
async fn targets_autocomplete_finds_by_slug() {
    let app = make_app().await;
    let v = get_json(app, "/api/targets/autocomplete?q=m31").await;
    let items = v["targets"].as_array().unwrap();
    assert_eq!(items.len(), 1, "expected exactly 1 result, got: {items:?}");
    assert_eq!(items[0]["slug"], "m31");
    // Response shape check: all three fields present.
    assert!(items[0]["canonical_name"].is_string());
    assert!(items[0]["kind"].is_string());
}

/// ?q= (empty string) returns empty targets array without hitting the DB.
#[tokio::test]
async fn targets_autocomplete_empty_q_returns_empty() {
    let app = make_app().await;
    let v = get_json(app, "/api/targets/autocomplete?q=").await;
    let items = v["targets"].as_array().unwrap();
    assert!(items.is_empty(), "expected empty array for empty q");
}

/// ?q=Seven Sisters matches m45 via aliases array (ILIKE on unnest).
#[tokio::test]
async fn targets_autocomplete_finds_by_alias_array() {
    let app = make_app().await;
    let v = get_json(app, "/api/targets/autocomplete?q=Seven+Sisters").await;
    let items = v["targets"].as_array().unwrap();
    assert!(
        items.iter().any(|t| t["slug"] == "m45"),
        "expected m45 in results via alias 'Seven Sisters', got: {items:?}"
    );
}
