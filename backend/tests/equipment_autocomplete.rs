//! Integration tests for Task 36: GET /api/equipment/autocomplete?kind=&q=
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

async fn make_app() -> (axum::Router, sqlx::PgPool) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    // Hold container alive for the test scope.
    std::mem::forget(pg);
    let router = http::router(pool.clone(), config_for(&url), storage, Arc::new(mailer));
    (router, pool)
}

async fn get_response(app: axum::Router, uri: &str) -> axum::http::Response<axum::body::Body> {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn get_json(app: axum::Router, uri: &str) -> serde_json::Value {
    let resp = get_response(app, uri).await;
    assert_eq!(resp.status(), 200, "expected 200 for {uri}");
    let bytes = axum::body::to_bytes(resp.into_body(), 65_536)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// GET ?kind=camera&q=ZWO returns the ZWO camera row.
#[tokio::test]
async fn finds_camera_by_display_name() {
    let (app, pool) = make_app().await;

    // Insert test rows.
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) values ($1, $2, $3, $4)",
        "camera", "zwo asi2600mc", "ZWO ASI2600MC", 10_i32
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) values ($1, $2, $3, $4)",
        "camera", "canon r6", "Canon R6", 5_i32
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) values ($1, $2, $3, $4)",
        "telescope", "redcat 51", "RedCat 51", 8_i32
    )
    .execute(&pool)
    .await
    .unwrap();

    let v = get_json(app, "/api/equipment/autocomplete?kind=camera&q=ZWO").await;
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "expected exactly 1 result, got: {items:?}");
    assert_eq!(
        items[0]["canonical_name"], "zwo asi2600mc",
        "expected zwo asi2600mc"
    );
    assert_eq!(items[0]["display_name"], "ZWO ASI2600MC");
    assert_eq!(items[0]["usage_count"], 10);
}

/// GET ?kind=camera&q= (empty q) returns empty items without DB query.
#[tokio::test]
async fn empty_q_returns_empty() {
    let (app, _pool) = make_app().await;
    let v = get_json(app, "/api/equipment/autocomplete?kind=camera&q=").await;
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty(), "expected empty array for empty q");
}

/// GET ?kind=foo&q=x returns 422 Validation.
#[tokio::test]
async fn invalid_kind_returns_422() {
    let (app, _pool) = make_app().await;
    let resp = get_response(app, "/api/equipment/autocomplete?kind=foo&q=x").await;
    assert_eq!(
        resp.status(),
        422,
        "expected 422 for invalid kind, got {}",
        resp.status()
    );
}

/// GET ?kind=camera&q=x with no matching rows returns empty items.
#[tokio::test]
async fn no_match_returns_empty() {
    let (app, _pool) = make_app().await;
    let v = get_json(
        app,
        "/api/equipment/autocomplete?kind=camera&q=NoMatchXYZ123",
    )
    .await;
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty(), "expected empty array for no match");
}

/// Higher usage_count comes first when multiple items match same query.
#[tokio::test]
async fn results_ordered_by_usage_count_desc() {
    let (app, pool) = make_app().await;

    // Insert two cameras both matching "ASI".
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) values ($1, $2, $3, $4)",
        "camera", "zwo asi533mc pro", "ZWO ASI533MC Pro", 3_i32
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count) values ($1, $2, $3, $4)",
        "camera", "zwo asi2600mc", "ZWO ASI2600MC", 15_i32
    )
    .execute(&pool)
    .await
    .unwrap();

    let v = get_json(app, "/api/equipment/autocomplete?kind=camera&q=ASI").await;
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "expected 2 results, got: {items:?}");
    // Higher usage_count (15) should be first.
    assert_eq!(
        items[0]["canonical_name"], "zwo asi2600mc",
        "expected higher usage_count first"
    );
    assert_eq!(items[0]["usage_count"], 15);
    assert_eq!(items[1]["canonical_name"], "zwo asi533mc pro");
    assert_eq!(items[1]["usage_count"], 3);
}

/// All five valid kinds are accepted (no 422).
#[tokio::test]
async fn all_valid_kinds_accepted() {
    for kind in &["telescope", "camera", "mount", "filter", "focal_modifier"] {
        let (app, _pool) = make_app().await;
        let uri = format!("/api/equipment/autocomplete?kind={kind}&q=nothing");
        let v = get_json(app, &uri).await;
        assert!(
            v["items"].is_array(),
            "kind={kind} should be accepted and return items array"
        );
    }
}

/// guiding is free-text only; the autocomplete endpoint must reject it with 422.
#[tokio::test]
async fn guiding_kind_is_no_longer_supported() {
    let (app, _pool) = make_app().await;
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/equipment/autocomplete?kind=guiding&q=foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}

/// focal_modifier kind is supported and returns matching items.
#[tokio::test]
async fn focal_modifier_kind_is_supported() {
    let (app, pool) = make_app().await;
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('focal_modifier','antares 0.7x reducer','Antares 0.7x Reducer',3)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let resp = get_response(app, "/api/equipment/autocomplete?kind=focal_modifier&q=red").await;
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 65_536)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let names: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["display_name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "Antares 0.7x Reducer"));
}
