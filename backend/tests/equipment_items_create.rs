//! Integration tests for Task 4: POST /api/equipment/items resolve-or-create
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::{Request, header}};
use http_body_util::BodyExt as _;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;

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

async fn make_app_and_pool() -> (axum::Router, sqlx::PgPool) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    std::mem::forget(pg);
    let router = http::router(pool.clone(), config_for(&url), Arc::new(MemoryStorage::new()), Arc::new(mailer));
    (router, pool)
}

async fn signup_and_cookie(app: &axum::Router, email: &str, handle: &str) -> String {
    let body = serde_json::json!({
        "email": email,
        "password": "verylongpassword",
        "display_name": "Test User",
        "handle": handle
    });
    let resp = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/auth/signup")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), 201, "signup failed");
    resp.headers().get("set-cookie").unwrap().to_str().unwrap().to_string()
}

#[tokio::test]
async fn insert_on_miss_returns_row_with_zero_count() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(r#"{"kind":"telescope","display_name":"Sky-Watcher 200P"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["display_name"], "Sky-Watcher 200P");
    assert_eq!(body["canonical_name"], "sky-watcher 200p");
    assert_eq!(body["kind"], "telescope");
    let count: i32 = sqlx::query_scalar!(
        "select usage_count from equipment_items where kind='telescope' and canonical_name='sky-watcher 200p'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn idempotent_on_hit_does_not_increment() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "bob@example.com", "bob1").await;
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope', 'celestron c8', 'Celestron C8', 7)"
    ).execute(&pool).await.unwrap();
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(r#"{"kind":"telescope","display_name":"Celestron C8"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let count: i32 = sqlx::query_scalar!(
        "select usage_count from equipment_items where kind='telescope' and canonical_name='celestron c8'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 7);
}

#[tokio::test]
async fn invalid_kind_returns_422() {
    let (app, _pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "carol@example.com", "carol1").await;
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(r#"{"kind":"banana","display_name":"x"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 422);
}
