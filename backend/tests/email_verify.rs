//! Integration tests for POST /api/auth/verify-email.
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
use serde_json::{Value, json};
use sha2::Digest;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

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

/// Boot a test stack. Returns (router, pool, outbox, container).
/// The caller MUST hold the returned `ContainerAsync` for the duration of the
/// test — dropping it tears down the Postgres container and causes pool timeouts.
async fn boot() -> (
    axum::Router,
    sqlx::PgPool,
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
    let app = http::router(pool.clone(), cfg, storage, Arc::new(mailer)).layer(MockConnectInfo(
        std::net::SocketAddr::from(([127, 0, 0, 1], 9999)),
    ));
    (app, pool, outbox, pg)
}

fn req_with_ip(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Insert a bare user row (no password). Returns the new user id.
async fn insert_user(pool: &sqlx::PgPool, email: &str, handle: &str) -> Uuid {
    astrophoto::users::queries::create_with_password(pool, email, handle, "Test User", "hash")
        .await
        .unwrap()
        .id
}

/// Insert a token row for the given user. `expires_offset_secs` is relative to
/// now: positive = future (valid), negative = past (expired).
async fn insert_token(pool: &sqlx::PgPool, user_id: Uuid, token: &str, expires_offset_secs: i64) {
    let hash: Vec<u8> = sha2::Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query!(
        "insert into email_verification_tokens (token_hash, user_id, expires_at)
          values ($1, $2, now() + make_interval(secs => $3))",
        hash,
        user_id,
        expires_offset_secs as f64
    )
    .execute(pool)
    .await
    .unwrap();
}

// ── tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn verify_with_unknown_token_returns_gone() {
    let (app, _pool, _outbox, _pg) = boot().await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/verify-email",
            json!({"token": "this-token-does-not-exist-in-db"}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::GONE);
}

#[tokio::test]
async fn verify_with_used_token_returns_gone() {
    let (app, pool, _outbox, _pg) = boot().await;

    let user_id = insert_user(&pool, "used@example.com", "used-tok").await;
    let token = "used-token-abc123";
    insert_token(&pool, user_id, token, 3600).await;

    // Mark it as used
    let hash: Vec<u8> = sha2::Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query!(
        "update email_verification_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&pool)
    .await
    .unwrap();

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/verify-email",
            json!({"token": token}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::GONE);
}

#[tokio::test]
async fn verify_with_expired_token_returns_gone() {
    let (app, pool, _outbox, _pg) = boot().await;

    let user_id = insert_user(&pool, "expired@example.com", "expired-tok").await;
    let token = "expired-token-xyz999";
    // expires 1 hour in the past
    insert_token(&pool, user_id, token, -3600).await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/verify-email",
            json!({"token": token}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::GONE);
}

#[tokio::test]
async fn verify_marks_user_verified_and_sets_session_cookie() {
    let (app, pool, _outbox, _pg) = boot().await;

    let user_id = insert_user(&pool, "verify@example.com", "verify-tok").await;
    let token = "valid-token-for-verification";
    // expires 1 hour in the future
    insert_token(&pool, user_id, token, 3600).await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/verify-email",
            json!({"token": token}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Check Set-Cookie header contains a session cookie
    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .expect("expected Set-Cookie header")
        .to_str()
        .unwrap();
    assert!(
        set_cookie.starts_with("session="),
        "expected cookie to start with 'session=', got: {set_cookie}"
    );

    // Verify the user's email_verified_at is now set
    let verified_at = sqlx::query_scalar!(
        "select email_verified_at from users where id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(verified_at.is_some(), "email_verified_at should be set after verification");

    // Verify the token's used_at is now set
    let hash: Vec<u8> = sha2::Sha256::digest(token.as_bytes()).to_vec();
    let used_at = sqlx::query_scalar!(
        "select used_at from email_verification_tokens where token_hash = $1",
        hash
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(used_at.is_some(), "token used_at should be set after verification");
}

#[tokio::test]
async fn resend_for_unverified_user_issues_new_token_and_sends_mail() {
    let (app, pool, outbox, _pg) = boot().await;

    let user_id = insert_user(&pool, "resend-unverified@example.com", "resend-unverified").await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/resend-verification",
            json!({"email": "resend-unverified@example.com"}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // One token row should exist for this user
    let token_count = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(token_count, 1, "expected exactly one token row");

    // One mail should have been sent
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1, "expected exactly one mail in outbox");
    assert_eq!(
        sent[0].subject,
        "Confirm your Astrophoto account",
        "unexpected subject"
    );
    assert!(
        sent[0].body.contains("/verify/"),
        "expected body to contain /verify/"
    );
}

#[tokio::test]
async fn resend_for_already_verified_user_is_silent_204_no_token_no_mail() {
    let (app, pool, outbox, _pg) = boot().await;

    let user_id = insert_user(&pool, "already-verified@example.com", "already-verified").await;
    // Mark the user as verified
    sqlx::query!(
        "update users set email_verified_at = now() where id = $1",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/resend-verification",
            json!({"email": "already-verified@example.com"}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // No token rows
    let token_count = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(token_count, 0, "expected no token rows for verified user");

    // No mail
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 0, "expected no mail for verified user");
}

#[tokio::test]
async fn resend_for_unknown_email_is_silent_204() {
    let (app, _pool, outbox, _pg) = boot().await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/resend-verification",
            json!({"email": "nobody@example.com"}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 0, "expected no mail for unknown email");
}

#[tokio::test]
async fn resend_within_cooldown_does_not_issue_token() {
    let (app, pool, outbox, _pg) = boot().await;

    let user_id =
        insert_user(&pool, "resend-cooldown@example.com", "resend-cooldown").await;

    // Insert a token that was created just now (within the 60s cooldown)
    insert_token(&pool, user_id, "existing-token-abc", 86400).await;

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/resend-verification",
            json!({"email": "resend-cooldown@example.com"}),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Still only one token row (no new one issued)
    let token_count = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(token_count, 1, "expected no new token due to cooldown");

    // No mail sent
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 0, "expected no mail within cooldown window");
}
