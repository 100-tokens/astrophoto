use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use testcontainers::ImageExt;
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
        platesolve_base_url: None,
        platesolve_api_key: None,
        platesolve_timeout_secs: 90,
    }
}

#[allow(clippy::unwrap_used)]
async fn signup_and_get_cookie(app: &axum::Router, pool: &sqlx::PgPool, email: &str) -> String {
    let handle = email.split('@').next().unwrap_or("user");
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
    assert_eq!(resp.status(), 202, "signup should succeed");

    // Mark user verified so login works.
    sqlx::query!(
        "update users set email_verified_at = now() where email = $1",
        email
    )
    .execute(pool)
    .await
    .unwrap();

    // Log in to get the session cookie.
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
    assert_eq!(login_resp.status(), 200, "login must succeed after signup");
    login_resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn upload_init_signs_url_and_dedups() {
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

    let cookie = signup_and_get_cookie(&app, &pool, "marie@example.com").await;

    // --- Happy path: one valid JPEG ----------------------------------------
    let body = serde_json::json!({
        "files": [{"name": "a.jpg", "size": 10485760, "mime": "image/jpeg", "hash": "abcdef"}]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "first upload init should return 200");
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        v["files"][0]["presigned_put_url"].is_string(),
        "presigned_put_url missing; got: {v}"
    );
    assert!(
        v["files"][0]["photo_id"].is_string(),
        "photo_id missing; got: {v}"
    );
    assert!(
        v["files"][0]["short_id"].is_string(),
        "short_id missing; got: {v}"
    );

    // --- Dedup: same hash for same owner must 409 --------------------------
    let r2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), 409, "duplicate hash should return 409");

    // --- Quota: free-tier file too large must 413 --------------------------
    let big = serde_json::json!({
        "files": [{"name": "b.jpg", "size": 62914560, "mime": "image/jpeg", "hash": "x"}]
    });
    let r3 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(big.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r3.status(), 413, "oversized file should return 413");
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn upload_init_rejects_xisf_when_platesolve_unconfigured() {
    // XISF is opt-in primary upload — it requires the plate-solve
    // service for the auto-calibrate trigger. Without that, an XISF
    // upload would sit in `awaiting-calibration` forever. The
    // upload_init gate should reject early so the user gets a clean
    // 400 instead of a stuck row.
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
    // `config_for` has platesolve_base_url=None, so http::router
    // builds AppState with platesolve=None.
    let app = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        None,
    );

    let cookie = signup_and_get_cookie(&app, &pool, "xisfgate@example.com").await;

    let body = serde_json::json!({
        "files": [{
            "name": "master.xisf",
            "size": 10_000_000,
            "mime": "application/x-xisf",
            "hash": "xisf-no-platesolve-test"
        }]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        400,
        "XISF mime without configured plate-solve must 400 (UnsupportedFormat)"
    );
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msg = v["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("application/x-xisf"),
        "error message should name the rejected mime; got: {msg}"
    );
    assert!(
        msg.contains("not configured"),
        "error message should explain why XISF was rejected; got: {msg}"
    );
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn upload_init_accepts_xisf_when_platesolve_configured() {
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
    let mut cfg = config_for(&url);
    cfg.platesolve_base_url = Some("http://127.0.0.1:1".into()); // unreachable, never called by init
    cfg.platesolve_api_key = Some("test-key".into());
    let client = astrophoto::photos::platesolve::PlatesolveClient::from_config(&cfg)
        .unwrap()
        .map(Arc::new);
    let app = http::router(
        pool.clone(),
        cfg,
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
        client,
    );

    let cookie = signup_and_get_cookie(&app, &pool, "xisfok@example.com").await;

    let body = serde_json::json!({
        "files": [{
            "name": "master.xisf",
            "size": 10_000_000,
            "mime": "application/x-xisf",
            "hash": "xisf-with-platesolve-test"
        }]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        200,
        "XISF mime with configured plate-solve must 200 (returns presigned PUT)"
    );
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        v["files"][0]["presigned_put_url"].is_string(),
        "XISF upload init should return a presigned PUT URL; got: {v}"
    );
}
