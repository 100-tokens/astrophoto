use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    body::Body,
    http::{Request, header},
};
use bytes::Bytes;
use http_body_util::BodyExt as _;
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

/// Sign up a user, mark them verified, log in, and return the `Set-Cookie` header value.
#[allow(clippy::unwrap_used)]
async fn signup_and_get_cookie(
    app: &axum::Router,
    pool: &sqlx::PgPool,
    email: &str,
    handle: &str,
) -> String {
    let body = serde_json::json!({
        "email": email,
        "password": "verylongpassword",
        "display_name": "Test User",
        "handle": handle
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
    assert_eq!(resp.status(), 202, "signup should succeed for {email}");

    // Mark user verified so login works.
    sqlx::query!(
        "update users set email_verified_at = now() where email = $1",
        email
    )
    .execute(pool)
    .await
    .unwrap();

    // Log in to obtain a session cookie.
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
    login_resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// Insert a photos row in `status='pending'` and return its id.
#[allow(clippy::unwrap_used)]
async fn insert_pending_photo(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    storage_key: &str,
    mime: &str,
    original_hash: &str,
) -> Uuid {
    let short_id = format!("T{}", &storage_key[..7.min(storage_key.len())]);
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        insert into photos
            (id, owner_id, storage_key, original_name, bytes, mime,
             original_hash, short_id, status, last_step, original_uploaded_at)
        values ($1, $2, $3, 'test.jpg', 1000, $4, $5, $6, 'pending', 'upload', now())
        "#,
        id,
        owner_id,
        storage_key,
        mime,
        original_hash,
        short_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// POST /api/uploads/:id/finalize with the given cookie and return the response.
#[allow(clippy::unwrap_used)]
async fn finalize(app: &axum::Router, photo_id: Uuid, cookie: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/uploads/{photo_id}/finalize"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn finalize_404_unknown_photo() {
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

    let cookie = signup_and_get_cookie(&app, &pool, "alice1@example.com", "alice1").await;

    // A random UUID that doesn't exist in the DB.
    let unknown_id = Uuid::new_v4();
    let resp = finalize(&app, unknown_id, &cookie).await;
    assert_eq!(resp.status(), 404, "unknown photo should return 404");
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn finalize_404_cross_owner() {
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

    // Sign up two users.
    let _cookie_alice = signup_and_get_cookie(&app, &pool, "alice2@example.com", "alice2").await;
    let cookie_bob = signup_and_get_cookie(&app, &pool, "bob2@example.com", "bob2").await;

    // Get Alice's user id from the DB.
    let alice_id: Uuid =
        sqlx::query_scalar!("select id from users where email = 'alice2@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // Insert a photo owned by Alice.
    let photo_id = insert_pending_photo(
        &pool,
        alice_id,
        "originals/cross-owner-test",
        "image/jpeg",
        "hash-cross-owner",
    )
    .await;

    // Bob tries to finalize Alice's photo — must see 404 (not 403).
    let resp = finalize(&app, photo_id, &cookie_bob).await;
    assert_eq!(
        resp.status(),
        404,
        "cross-owner finalize should return 404 (leak prevention)"
    );
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn finalize_408_no_s3_object() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::clone(&storage),
        Arc::new(mailer),
    );

    let cookie = signup_and_get_cookie(&app, &pool, "alice3@example.com", "alice3").await;
    let alice_id: Uuid =
        sqlx::query_scalar!("select id from users where email = 'alice3@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // Photo row exists but nothing was PUT to storage.
    let photo_id = insert_pending_photo(
        &pool,
        alice_id,
        "originals/no-s3-object",
        "image/jpeg",
        "hash-no-s3",
    )
    .await;

    let resp = finalize(&app, photo_id, &cookie).await;
    assert_eq!(
        resp.status(),
        408,
        "missing S3 object should return 408 (PendingFinalizeStuck)"
    );
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["error"], "pending-finalize-stuck");
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn finalize_400_magic_byte_mismatch() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::clone(&storage),
        Arc::new(mailer),
    );

    let cookie = signup_and_get_cookie(&app, &pool, "alice4@example.com", "alice4").await;
    let alice_id: Uuid =
        sqlx::query_scalar!("select id from users where email = 'alice4@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();

    let key = "originals/magic-mismatch";
    // Row declared as JPEG but we seed PNG bytes.
    let photo_id = insert_pending_photo(&pool, alice_id, key, "image/jpeg", "hash-magic").await;

    // PNG magic bytes — sniff returns Png, but mime is image/jpeg → mismatch.
    let png_header = Bytes::from_static(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR");
    storage.put(key, "image/png", png_header).await.unwrap();

    let resp = finalize(&app, photo_id, &cookie).await;
    assert_eq!(resp.status(), 400, "magic-byte mismatch should return 400");
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["error"], "magic-byte-mismatch");

    // The photo status should now be 'failed' with pipeline_error set.
    let row = sqlx::query!(
        "select status, pipeline_error from photos where id = $1",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.status, "failed");
    assert!(
        row.pipeline_error
            .as_deref()
            .unwrap_or("")
            .contains("magic-byte"),
        "pipeline_error should mention magic-byte mismatch"
    );
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn finalize_happy_path_and_idempotent() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let storage: Arc<dyn astrophoto::storage::Storage> = Arc::new(MemoryStorage::new());
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::clone(&storage),
        Arc::new(mailer),
    );

    let cookie = signup_and_get_cookie(&app, &pool, "alice5@example.com", "alice5").await;
    let alice_id: Uuid =
        sqlx::query_scalar!("select id from users where email = 'alice5@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();

    let key = "originals/happy-path";
    let photo_id = insert_pending_photo(&pool, alice_id, key, "image/jpeg", "hash-happy").await;

    // Seed a real JPEG fixture into storage.
    let jpeg_bytes = Bytes::from_static(include_bytes!("fixtures/sample.jpg"));
    storage.put(key, "image/jpeg", jpeg_bytes).await.unwrap();

    // First call — should finalize and return ready.
    let resp = finalize(&app, photo_id, &cookie).await;
    assert_eq!(resp.status(), 200, "happy path should return 200");
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["status"], "ready");
    assert!(
        v["display_key"].is_string(),
        "display_key should be populated after finalize; got: {v}"
    );

    // Confirm DB status.
    let status: String = sqlx::query_scalar!("select status from photos where id = $1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "ready");

    // Second call — idempotent, must return 200 without re-running the pipeline.
    let resp2 = finalize(&app, photo_id, &cookie).await;
    assert_eq!(
        resp2.status(),
        200,
        "second finalize call should return 200"
    );
    let body_bytes2 = resp2.into_body().collect().await.unwrap().to_bytes();
    let v2: serde_json::Value = serde_json::from_slice(&body_bytes2).unwrap();
    assert_eq!(v2["status"], "ready");
    assert!(
        v2["display_key"].is_string(),
        "display_key should still be populated on idempotent call; got: {v2}"
    );
}
