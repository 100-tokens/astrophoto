/// Integration test for the dev CDN route (`GET /cdn/img/:id`).
///
/// Uses MemoryStorage in lieu of MinIO — no S3 container needed.
/// The config's `cdn_base_url` contains "localhost", so the conditional
/// mount in `http::router` is active.
use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use bytes::Bytes;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

#[allow(clippy::expect_used)]
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

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn dev_cdn_resizes_display_master() {
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

    // Hold an Arc<dyn Storage> so we can call .put() after passing a clone
    // into the router (which consumes its argument).
    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(pool, config_for(&url), storage.clone(), Arc::new(mailer));

    let photo_id = Uuid::new_v4();
    let bytes: &[u8] = include_bytes!("fixtures/sample.jpg");

    storage
        .put(
            &format!("display/{photo_id}.jpg"),
            "image/jpeg",
            Bytes::copy_from_slice(bytes),
        )
        .await
        .unwrap();

    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/cdn/img/{photo_id}?w=100"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers().get("content-type").unwrap().to_str().unwrap(),
        "image/jpeg"
    );
    let resized = axum::body::to_bytes(r.into_body(), 1_000_000)
        .await
        .unwrap();
    assert!(
        resized.len() < bytes.len(),
        "resized ({} bytes) should be smaller than original ({} bytes)",
        resized.len(),
        bytes.len()
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn dev_cdn_returns_404_for_missing_master() {
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

    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(pool, config_for(&url), storage, Arc::new(mailer));

    let missing_id = Uuid::new_v4();
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/cdn/img/{missing_id}?w=400"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), 404);
}
