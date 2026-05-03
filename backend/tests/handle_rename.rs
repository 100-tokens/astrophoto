use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    body::Body,
    http::{Request, header},
};
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
    }
}

#[tokio::test]
async fn rename_handle_writes_redirect_row() {
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

    // 1. Sign up as 'marie'.
    let signup_body = serde_json::json!({
        "email": "marie@example.com",
        "password": "verylongpassword",
        "display_name": "Marie Dubois",
        "handle": "marie"
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(signup_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Extract session cookie from signup response.
    let session_cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        session_cookie.starts_with("session="),
        "got: {session_cookie}"
    );

    // Get user_id by querying the pool.
    let user_id: uuid::Uuid =
        sqlx::query_scalar("select id from users where email = 'marie@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // 2. POST /api/me/handle to rename to 'marie2'.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &session_cookie)
                .body(Body::from(
                    serde_json::json!({"handle": "marie2"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // 3. Redirect row exists for the old handle.
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from handle_redirects where old_handle = 'marie')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists, "handle_redirects row for 'marie' should exist");

    // 4. User's handle is now 'marie2'.
    let new_handle: String = sqlx::query_scalar("select handle::text from users where id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(new_handle, "marie2");
}

#[tokio::test]
async fn rename_handle_same_handle_returns_204() {
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

    // Sign up as 'astro'.
    let signup_body = serde_json::json!({
        "email": "astro@example.com",
        "password": "verylongpassword",
        "display_name": "Astro",
        "handle": "astro"
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(signup_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let session_cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // POST the same handle — should be a no-op returning 204.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &session_cookie)
                .body(Body::from(
                    serde_json::json!({"handle": "astro"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // No redirect row should have been written.
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from handle_redirects where old_handle = 'astro')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !exists,
        "no redirect row should exist for a same-handle no-op"
    );
}

#[tokio::test]
async fn rename_handle_conflict_returns_409() {
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

    // Sign up 'user1'.
    let signup_body = serde_json::json!({
        "email": "user1@example.com",
        "password": "verylongpassword",
        "display_name": "User One",
        "handle": "userone"
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(signup_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let session_cookie = r
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Sign up 'user2' who already holds the handle 'usertwo'.
    let signup_body2 = serde_json::json!({
        "email": "user2@example.com",
        "password": "verylongpassword",
        "display_name": "User Two",
        "handle": "usertwo"
    });
    let r2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(signup_body2.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), 201);

    // user1 tries to rename to 'usertwo' — should 409.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &session_cookie)
                .body(Body::from(
                    serde_json::json!({"handle": "usertwo"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
}

#[tokio::test]
async fn rename_handle_unauthenticated_returns_401() {
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

    // No cookie provided.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"handle": "anynewhandle"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn rename_handle_invalid_format_returns_422() {
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

    // Sign up a user.
    let signup_body = serde_json::json!({
        "email": "valid@example.com",
        "password": "verylongpassword",
        "display_name": "Valid",
        "handle": "validuser"
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(signup_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let session_cookie = r
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Too short — invalid format.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/me/handle")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &session_cookie)
                .body(Body::from(serde_json::json!({"handle": "ab"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);
}
