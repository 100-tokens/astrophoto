use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
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
        platesolve_base_url: None,
        platesolve_api_key: None,
        platesolve_timeout_secs: 90,
    }
}

#[tokio::test]
async fn healthz_returns_ok_with_real_postgres() {
    let pg = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
        .expect("postgres container failed to start");
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = db::connect(&url).await.expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool,
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        None,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["db"], "ok");
}
