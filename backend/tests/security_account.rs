//! Integration tests for Phase 8a security & account flows.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::sync::Mutex;

use astrophoto::mail::{Mailer, SentMail};
use astrophoto::storage::MemoryStorage;
use astrophoto::{Config, db, http};
use axum::{
    body::Body,
    extract::connect_info::MockConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::json;
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
        public_base_url: "http://localhost:5173".into(),
        s3_endpoint: None,
        s3_region: "us-east-1".into(),
        s3_bucket: "x".into(),
        s3_access_key: "a".into(),
        s3_secret_key: "s".into(),
        s3_path_style: true,
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

/// Boot a test stack. Returns (router, outbox, container).
/// The caller must hold `_pg` for the duration of the test — dropping it tears
/// down the Postgres container and causes pool timeouts.
async fn boot() -> (
    axum::Router,
    Arc<Mutex<Vec<SentMail>>>,
    testcontainers::ContainerAsync<PgImage>,
) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let cfg = config_for(&url);
    let storage = Arc::new(MemoryStorage::new());
    let (mailer, outbox) = Mailer::for_test();
    // Wrap with MockConnectInfo so ConnectInfo<SocketAddr> extracts correctly
    // without needing a real TCP listener.
    let app = http::router(pool, cfg, storage, Arc::new(mailer))
        .layer(MockConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 9999))));
    (app, outbox, pg)
}

async fn signup(app: &axum::Router, email: &str, password: &str) {
    let body = json!({"email": email, "password": password, "display_name": "Marie"});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "signup must succeed (got {})",
        resp.status()
    );
}

fn reset_request(email: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/auth/password-reset/request")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"email": email}).to_string()))
        .unwrap()
}

#[tokio::test]
async fn password_reset_request_unknown_email_returns_204_silent() {
    let (app, outbox, _pg) = boot().await;
    let resp = app
        .oneshot(reset_request("ghost@nowhere.test"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(
        outbox.lock().unwrap().is_empty(),
        "no mail must be sent for unknown emails"
    );
}

#[tokio::test]
async fn password_reset_request_known_email_sends_one_mail() {
    let (app, outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    let resp = app
        .clone()
        .oneshot(reset_request("marie@example.com"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "marie@example.com");
    assert!(sent[0].subject.contains("Reset"));
    assert!(sent[0].body.contains("/reset/"));
}

#[tokio::test]
async fn password_reset_throttle_60s_per_email() {
    let (app, outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    for _ in 0..3 {
        let resp = app
            .clone()
            .oneshot(reset_request("marie@example.com"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
    assert_eq!(
        outbox.lock().unwrap().len(),
        1,
        "only the first request emails"
    );
}

// (The OAuth-only "set a password" template path is exercised in Task 6,
//  once `boot()` exposes `pool` and we can `INSERT INTO users (..., password_hash) VALUES (..., NULL)`
//  directly.)
