use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::Request};
use testcontainers::ImageExt;
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

/// Published photo redirects to the canonical permalink with 308.
///
/// axum 0.7's `Redirect::permanent` returns HTTP 308 (Permanent Redirect),
/// not 301. Both instruct clients to cache and reuse the new URL; 308 is the
/// modern, method-preserving counterpart to 301.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn redirect_published_photo_returns_308_to_canonical() {
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

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        None,
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
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/by-uuid/{photo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // axum 0.7 Redirect::permanent → 308 Permanent Redirect
    assert_eq!(r.status(), axum::http::StatusCode::PERMANENT_REDIRECT);
    let location = r.headers().get(axum::http::header::LOCATION).unwrap();
    assert_eq!(location.to_str().unwrap(), "/u/marie/p/ABCD1234");
}

/// Unknown UUID must return 404 (not a 400 from path parsing failure).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn redirect_unknown_uuid_returns_404() {
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

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        None,
    );

    // A random UUID that has never been inserted — DB returns None → 404.
    let unknown = Uuid::new_v4();

    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/by-uuid/{unknown}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), axum::http::StatusCode::NOT_FOUND);
}

/// Draft photo (published_at IS NULL) must NOT be redirectable.
/// The SQL filter `published_at is not null` ensures drafts return 404.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn redirect_draft_photo_returns_404() {
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

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        None,
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

    // published_at intentionally omitted — stays NULL (draft state).
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
                .uri(format!("/api/photos/by-uuid/{photo_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r.status(), axum::http::StatusCode::NOT_FOUND);
}
