//! GET /api/auth/handle-check?handle=foo
//! Returns 200 with {"status":"available"|"taken"|"reserved"|"invalid"}.

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::body::Body;
use axum::http::Request;
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

#[tokio::test]
async fn handle_check_returns_available_then_taken() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
    );

    // Available
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/handle-check?handle=fresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["status"], "available");

    // Reserved
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/handle-check?handle=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["status"], "reserved");

    // Invalid
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/handle-check?handle=AB")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body["status"], "invalid");
}
