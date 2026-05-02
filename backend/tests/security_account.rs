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
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::{Value, json};
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

/// Boot a test stack. Returns (router, pool, outbox, container).
/// The caller MUST hold the returned `ContainerAsync` for the duration of the
/// test — dropping it tears down the Postgres container and causes pool
/// timeouts.
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
    // Wrap with MockConnectInfo so ConnectInfo<SocketAddr> extracts correctly
    // without needing a real TCP listener.
    let app = http::router(pool.clone(), cfg, storage, Arc::new(mailer)).layer(MockConnectInfo(
        std::net::SocketAddr::from(([127, 0, 0, 1], 9999)),
    ));
    (app, pool, outbox, pg)
}

/// Build a JSON POST request. The app layer's MockConnectInfo handles the
/// ConnectInfo<SocketAddr> extraction for all routes.
fn req_with_ip(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
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

/// Sign in and return the full Set-Cookie header value (e.g. `session=...; HttpOnly; ...`).
async fn signin(app: &axum::Router, email: &str, password: &str) -> String {
    let resp = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/login",
            json!({"email": email, "password": password}),
        ))
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "signin must succeed (got {})",
        resp.status()
    );
    resp.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
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
    let (app, _pool, outbox, _pg) = boot().await;
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
    let (app, _pool, outbox, _pg) = boot().await;
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
    let (app, _pool, outbox, _pg) = boot().await;
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

#[tokio::test]
async fn password_reset_full_happy_path() {
    let (app, _pool, outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    // Request reset.
    app.clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/password-reset/request",
            json!({"email": "marie@example.com"}),
        ))
        .await
        .unwrap();

    let body = outbox.lock().unwrap()[0].body.clone();
    // Extract token from the reset link: http://localhost:5173/reset/<token>
    let token = body
        .split("/reset/")
        .nth(1)
        .expect("reset link in mail body")
        .split_whitespace()
        .next()
        .expect("token after /reset/");

    // Confirm with a strong new password.
    let resp = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/password-reset/confirm",
            json!({"token": token.trim(), "new_password": "evenlongerpw12"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let cookie = resp.headers().get(header::SET_COOKIE).unwrap();
    assert!(cookie.to_str().unwrap().contains("session"));

    // Old password no longer works.
    let resp_old = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/login",
            json!({"email": "marie@example.com", "password": "longenoughpw1"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp_old.status(), StatusCode::UNAUTHORIZED);

    // New password works.
    let resp_new = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/login",
            json!({"email": "marie@example.com", "password": "evenlongerpw12"}),
        ))
        .await
        .unwrap();
    assert!(resp_new.status().is_success());
}

#[tokio::test]
async fn password_reset_token_single_use() {
    let (app, _pool, outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    app.clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/password-reset/request",
            json!({"email": "marie@example.com"}),
        ))
        .await
        .unwrap();

    let token = outbox.lock().unwrap()[0]
        .body
        .split("/reset/")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    // First confirm: 204.
    let r1 = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/password-reset/confirm",
            json!({"token": token, "new_password": "evenlongerpw12"}),
        ))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::NO_CONTENT);

    // Second confirm with same token: 410 Gone.
    let r2 = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/password-reset/confirm",
            json!({"token": token, "new_password": "anotherlongerpw9"}),
        ))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::GONE);
}

#[tokio::test]
async fn password_reset_oauth_user_gets_set_password_template() {
    let (app, pool, outbox, _pg) = boot().await;
    // OAuth-only user: signup never happened; insert directly with NULL password_hash.
    sqlx::query!(
        "insert into users (email, display_name, password_hash) values ($1, $2, null)",
        "oauth@x.test",
        "OAuthie"
    )
    .execute(&pool)
    .await
    .unwrap();
    outbox.lock().unwrap().clear();

    app.oneshot(req_with_ip(
        "POST",
        "/api/auth/password-reset/request",
        json!({"email": "oauth@x.test"}),
    ))
    .await
    .unwrap();

    let sent = outbox.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert!(
        sent[0].subject.contains("Set a password"),
        "OAuth-only user should get the set-password subject, got: {}",
        sent[0].subject
    );
}

#[tokio::test]
async fn password_change_invalidates_all_sessions_then_creates_fresh() {
    let (app, pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    let cookie = signin(&app, "marie@example.com", "longenoughpw1").await;

    // Use the session cookie to change the password.
    let session_cookie = cookie.split(';').next().unwrap().to_string();
    let req = Request::builder()
        .method("POST")
        .uri("/api/me/password-change")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &session_cookie)
        .body(Body::from(
            json!({
                "current_password": "longenoughpw1",
                "new_password": "evenlongerpw12"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(
        resp.headers().contains_key(header::SET_COOKIE),
        "must rotate the cookie"
    );

    // Exactly one row in sessions for this user (the new rotated session).
    let count: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from sessions s join users u on u.id = s.user_id
          where u.email = $1"#,
        "marie@example.com"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn password_change_wrong_current_returns_401() {
    let (app, _pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    let cookie = signin(&app, "marie@example.com", "longenoughpw1").await;

    let session_cookie = cookie.split(';').next().unwrap().to_string();
    let req = Request::builder()
        .method("POST")
        .uri("/api/me/password-change")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &session_cookie)
        .body(Body::from(
            json!({
                "current_password": "WRONG",
                "new_password": "evenlongerpw12"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_change_full_happy_path() {
    let (app, pool, outbox, _pg) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;
    outbox.lock().unwrap().clear();

    let req = Request::builder()
        .method("POST")
        .uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(
            json!({"new_email": "marie@new.test", "current_password": "longenoughpw1"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sent = outbox.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "marie@new.test");
    let token = sent[0]
        .body
        .split("/email-change/")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/email-change/confirm",
            json!({"token": token}),
        ))
        .await
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["status"], "success");

    // Old address must receive the notification.
    let sent = outbox.lock().unwrap().clone();
    assert!(
        sent.iter()
            .any(|m| m.to == "marie@old.test" && m.subject.contains("changed"))
    );

    // Email row was actually updated.
    let row = sqlx::query!(
        "select email as \"email!: String\" from users where email = $1",
        "marie@new.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.email, "marie@new.test");
}

#[tokio::test]
async fn email_change_target_already_taken_returns_taken_status() {
    let (app, _pool, outbox, _pg) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    signup(&app, "leah@taken.test", "longenoughpw2").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;
    outbox.lock().unwrap().clear();

    let req = Request::builder()
        .method("POST")
        .uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(
            json!({"new_email": "leah@taken.test", "current_password": "longenoughpw1"})
                .to_string(),
        ))
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
    let token = outbox.lock().unwrap()[0]
        .body
        .split("/email-change/")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    let resp = app
        .oneshot(req_with_ip(
            "POST",
            "/api/auth/email-change/confirm",
            json!({"token": token}),
        ))
        .await
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["status"], "taken");
}

#[tokio::test]
async fn email_change_pending_token_invalidated_on_new_request() {
    let (app, pool, outbox, _pg) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;

    for new in ["a@x.test", "b@x.test"] {
        let req = Request::builder()
            .method("POST")
            .uri("/api/me/email-change/request")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, cookie.split(';').next().unwrap())
            .body(Body::from(
                json!({"new_email": new, "current_password": "longenoughpw1"}).to_string(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();
    }

    let active: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from email_change_tokens where used_at is null"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 1);
    let _ = outbox;
}

#[tokio::test]
async fn email_change_throttle_60s_per_user() {
    let (app, _pool, outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    // First request: 204
    let req = Request::builder()
        .method("POST")
        .uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(
            json!({"new_email": "a@x.test", "current_password": "longenoughpw1"}).to_string(),
        ))
        .unwrap();
    let r1 = app.clone().oneshot(req).await.unwrap();
    assert_eq!(r1.status(), StatusCode::NO_CONTENT);

    // Second request immediately: 429
    let req = Request::builder()
        .method("POST")
        .uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(
            json!({"new_email": "b@x.test", "current_password": "longenoughpw1"}).to_string(),
        ))
        .unwrap();
    let r2 = app.oneshot(req).await.unwrap();
    assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(outbox.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn email_change_oauth_only_user_blocked_400() {
    let (app, pool, _outbox, _pg) = boot().await;
    // Create OAuth-only user (no password) directly.
    let user_id: uuid::Uuid = sqlx::query_scalar!(
        "insert into users (email, display_name, password_hash) values ($1, 'OAuth', null) returning id",
        "oauth@x.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Mint a session row directly (cookie value = base64url(session.id), TTL 30 days).
    let mut sess_id = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut sess_id[..]);
    sqlx::query!(
        "insert into sessions (id, user_id, expires_at) values ($1, $2, now() + interval '30 days')",
        &sess_id[..], user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let cookie = format!(
        "session={}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sess_id)
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(
            json!({"new_email": "newaddr@x.test", "current_password": ""}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn profile_get_put_round_trip() {
    let (app, _pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    // PUT
    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/me/profile")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(
            json!({"display_name": "Marie Dubois"}).to_string(),
        ))
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    // GET
    let mut req = Request::builder()
        .method("GET")
        .uri("/api/me/profile")
        .header(header::COOKIE, &cookie_h)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["display_name"], "Marie Dubois");
}

#[tokio::test]
async fn preferences_default_dark_work_then_updated() {
    let (app, _pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    let mut req = Request::builder()
        .method("GET")
        .uri("/api/me/preferences")
        .header(header::COOKIE, &cookie_h)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.clone().oneshot(req).await.unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["theme"], "dark");
    assert_eq!(v["density"], "work");

    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/me/preferences")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(json!({"theme": "light"}).to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn sessions_list_marks_current_first() {
    let (app, pool, _outbox, _pg) = boot().await;
    // signup itself creates a session; signin creates one more — at least 2 total.
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_a = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _cookie_b = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder()
        .method("GET")
        .uri("/api/me/sessions")
        .header(header::COOKIE, cookie_a.split(';').next().unwrap())
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let arr = v.as_array().unwrap();
    // signup + 2 signins = at least 2 sessions; what matters is ordering.
    assert!(arr.len() >= 2);
    assert_eq!(arr[0]["is_current"], true);
    // Every other session must not be the current one.
    for row in &arr[1..] {
        assert_eq!(row["is_current"], false);
    }
    let _ = pool;
}

#[tokio::test]
async fn revoke_current_session_returns_400() {
    let (app, pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    // Extract the session token bytes from the cookie to get the actual current id.
    let token_b64 = cookie
        .split(';')
        .next()
        .unwrap()
        .trim()
        .split('=')
        .nth(1)
        .unwrap();
    let token_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token_b64)
        .unwrap();
    let id_hex = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&token_bytes);

    let mut req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/me/sessions/{id_hex}"))
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let _ = pool;
}

#[tokio::test]
async fn sign_out_others_keeps_current_kills_rest() {
    let (app, pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_a = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _ = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _ = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder()
        .method("POST")
        .uri("/api/me/sessions/sign-out-others")
        .header(header::COOKIE, cookie_a.split(';').next().unwrap())
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from sessions s join users u on u.id = s.user_id \
         where u.email = $1",
        "marie@x.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn delete_request_with_correct_password_and_phrase_succeeds() {
    let (app, pool, outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder()
        .method("POST")
        .uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(
            json!({
                "current_password": "longenoughpw1",
                "confirmation_phrase": "DELETE MY ACCOUNT"
            })
            .to_string(),
        ))
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    let pending: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
        "select pending_deletion_at from users where email = $1",
        "marie@x.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(pending.is_some());
    assert!(
        outbox
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.subject.contains("scheduled"))
    );
}

#[tokio::test]
async fn delete_request_wrong_phrase_returns_400() {
    let (app, _pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder()
        .method("POST")
        .uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(
            json!({
                "current_password": "longenoughpw1",
                "confirmation_phrase": "delete my account"
            })
            .to_string(),
        ))
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn delete_request_idempotent_does_not_extend_grace() {
    let (app, pool, _outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    for _ in 0..2 {
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/me/delete-request")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie_h)
            .body(Body::from(
                json!({
                    "current_password": "longenoughpw1",
                    "confirmation_phrase": "DELETE MY ACCOUNT"
                })
                .to_string(),
            ))
            .unwrap();
        req.extensions_mut()
            .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
        app.clone().oneshot(req).await.unwrap();
    }
    let pending: chrono::DateTime<chrono::Utc> = sqlx::query_scalar!(
        "select pending_deletion_at as \"p!: chrono::DateTime<chrono::Utc>\"
           from users where email = $1",
        "marie@x.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let dt = (pending - chrono::Utc::now()).num_hours();
    assert!(
        (167..169).contains(&dt),
        "grace must remain ~7 days, got {dt}h"
    );
}

#[tokio::test]
async fn delete_cancel_clears_pending_and_emails() {
    let (app, pool, outbox, _pg) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    // Mark for deletion.
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(
            json!({
                "current_password": "longenoughpw1",
                "confirmation_phrase": "DELETE MY ACCOUNT"
            })
            .to_string(),
        ))
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    app.clone().oneshot(req).await.unwrap();
    outbox.lock().unwrap().clear();

    // Cancel.
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/me/delete-cancel")
        .header(header::COOKIE, &cookie_h)
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    let pending: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
        "select pending_deletion_at from users where email = $1",
        "marie@x.test"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(pending.is_none());
    assert!(
        outbox
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.subject.contains("cancelled"))
    );
}
