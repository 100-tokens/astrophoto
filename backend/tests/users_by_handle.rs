//! Integration tests for:
//!   GET /api/users/by-handle/:handle
//!   GET /api/handles/redirect/:old_handle

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    body::Body,
    http::{Request, header},
};
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
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

async fn read_json(body: axum::body::Body) -> serde_json::Value {
    let bytes = axum::body::to_bytes(body, 64 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// Helper: sign up a user, mark verified, log in, return the session cookie.
async fn signup(
    app: axum::Router,
    pool: &sqlx::PgPool,
    email: &str,
    handle: &str,
    display_name: &str,
) -> (axum::Router, String) {
    let body = serde_json::json!({
        "email": email,
        "password": "verylongpassword",
        "display_name": display_name,
        "handle": handle,
    });
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
    assert_eq!(resp.status(), 202, "signup failed for {email}");

    // Mark user verified.
    sqlx::query!(
        "update users set email_verified_at = now() where email = $1",
        email
    )
    .execute(pool)
    .await
    .unwrap();

    // Log in to get a session cookie.
    let login_body = serde_json::json!({"email": email, "password": "verylongpassword"});
    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        login_resp.status(),
        200,
        "login must succeed after signup for {email}"
    );
    let cookie = login_resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    (app, cookie)
}

// ---------------------------------------------------------------------------
// by-handle hit: 200 with expected fields, photo_count = 0 for fresh user.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn by_handle_hit_returns_200_with_correct_shape() {
    let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
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

    let (app, _) = signup(app, &pool, "nova@example.com", "nova", "Nova Star").await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/users/by-handle/nova")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body = read_json(resp.into_body()).await;
    assert_eq!(body["handle"], "nova");
    assert_eq!(body["display_name"], "Nova Star");
    // Fresh user has no published photos.
    assert_eq!(body["photo_count"], 0);
    // id and created_at must be present.
    assert!(body["id"].is_string(), "id should be a string UUID");
    assert!(
        body["created_at"].is_string(),
        "created_at should be an RFC3339 string"
    );
    // email must NOT be exposed.
    assert!(
        body.get("email").is_none(),
        "email must not appear in response"
    );
}

// ---------------------------------------------------------------------------
// by-handle miss: unknown handle → 404.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn by_handle_miss_returns_404() {
    let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool,
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/users/by-handle/no-such-user")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

// ---------------------------------------------------------------------------
// redirect hit: signup → rename → GET old handle resolves to new handle.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn redirect_hit_returns_current_handle() {
    let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
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

    // Sign up as 'oldhandle', then rename to 'newhandle'.
    let (app, cookie) = signup(app, &pool, "astro@example.com", "oldhandle", "Astro").await;

    let rename_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    serde_json::json!({"handle": "newhandle"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rename_resp.status(), 200, "rename should succeed");

    // Redirect lookup on the old handle returns the new one.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/handles/redirect/oldhandle")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = read_json(resp.into_body()).await;
    assert_eq!(body["handle"], "newhandle");
}

// ---------------------------------------------------------------------------
// redirect miss: handle that has never existed → 404.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn redirect_miss_returns_404() {
    let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool,
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/handles/redirect/never-existed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}
