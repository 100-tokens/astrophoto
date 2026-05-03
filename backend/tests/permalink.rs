use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
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
#[allow(clippy::unwrap_used)]
async fn resolve_permalink_returns_photo_id() {
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

    let user_id = Uuid::new_v4();
    let photo_id = Uuid::new_v4();

    sqlx::query!(
        "insert into users (id, email, handle, display_name, password_hash) \
         values ($1, 'm@example.com', 'marie', 'M', 'x')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at, published_at) \
         values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', 'ABCD1234', now(), now())",
        photo_id,
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/photos/by-permalink/marie/ABCD1234")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(body["id"], photo_id.to_string());
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn resolve_permalink_unknown_short_id_returns_404() {
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

    let user_id = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, handle, display_name, password_hash) \
         values ($1, 'z@example.com', 'zara', 'Z', 'x')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .oneshot(
            Request::builder()
                .uri("/api/photos/by-permalink/zara/NOTEXIST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn resolve_permalink_draft_returns_404() {
    // A photo with published_at = NULL (draft) must NOT be resolvable via
    // the public permalink endpoint — only published photos are accessible.
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

    let user_id = Uuid::new_v4();
    let photo_id = Uuid::new_v4();

    sqlx::query!(
        "insert into users (id, email, handle, display_name, password_hash) \
         values ($1, 'd@example.com', 'draft_owner', 'D', 'x')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    // published_at is intentionally omitted — defaults to NULL (draft state).
    sqlx::query!(
        "insert into photos \
         (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
         values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', 'DRAFT001', now())",
        photo_id,
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .oneshot(
            Request::builder()
                .uri("/api/photos/by-permalink/draft_owner/DRAFT001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), 404);
}
