//! Shared test harness for backend integration tests.
//!
//! Cargo treats `tests/common/mod.rs` specially: it is NOT compiled as
//! its own test binary, so importing it via `mod common;` from sibling
//! test files works without duplicate-binary errors.
//!
//! IMPORTANT: When `session_secure=false` (always the case in tests),
//! the session cookie name drops the `__Host-` prefix and is plain
//! `session=` per `backend/src/auth/session.rs::cookie_name()`. Cookie
//! assertions everywhere depend on this.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use astrophoto::{Config, db, http, mail::Mailer, storage::MemoryStorage};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde::de::DeserializeOwned;
use serde_json::Value as Json;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

pub fn config_for(url: &str) -> Config {
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

/// Spin up a fresh Postgres container, run migrations, and return a
/// (Router, PgPool) tuple. The container is leaked via `std::mem::forget`
/// so it lives until process exit — identical to the pattern previously
/// used inline in equipment_* and photos_apply_setup test files.
pub async fn make_app_and_pool() -> (Router, sqlx::PgPool) {
    use astrophoto::mail::Mailer;
    let pg = testcontainers_modules::postgres::Postgres::default()
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = Mailer::for_test();
    std::mem::forget(pg);
    let router = astrophoto::http::router(
        pool.clone(),
        config_for(&url),
        std::sync::Arc::new(astrophoto::storage::MemoryStorage::new()),
        std::sync::Arc::new(mailer),
    );
    (router, pool)
}

/// POST /api/auth/signup and return the raw `set-cookie` header value.
pub async fn signup_and_cookie(app: &Router, email: &str, handle: &str) -> String {
    let body = serde_json::json!({
        "email": email,
        "password": "verylongpassword",
        "display_name": "Test User",
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
    assert_eq!(resp.status(), 201, "signup failed");
    resp.headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// Return the UUID of the user with the given email (must already exist).
pub async fn lookup_user_id(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!("select id from users where email = $1", email)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Insert a second user directly via SQL (avoids a full signup round-trip).
/// Useful for cross-user auth tests. Returns the new user's UUID.
pub async fn create_other_user(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"insert into users (email, password_hash, display_name, handle)
                values ($1, '$argon2id$v=19$m=19456,t=2,p=1$AAAA$AAAA', 'Other', $2)
           returning id"#,
        email,
        email.split('@').next().unwrap()
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Insert a minimal photo row. `setup_id`, `scope`, and `camera` are all
/// optional so callers can seed only what they need. Returns the photo UUID.
pub async fn insert_stub_photo(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    setup_id: Option<Uuid>,
    scope: Option<String>,
    camera: Option<String>,
) -> Uuid {
    sqlx::query_scalar!(
        r#"insert into photos
              (owner_id, storage_key, original_name, bytes, mime,
               original_uploaded_at, short_id, last_step,
               setup_id, scope, camera)
            values ($1, 'k', 'orig.jpg', 1000, 'image/jpeg',
                   now(), gen_random_uuid()::text, 'caption',
                   $2, $3, $4)
            returning id"#,
        owner_id,
        setup_id,
        scope.as_deref(),
        camera.as_deref()
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub struct TestApp {
    pub app: Router,
    pub pool: sqlx::PgPool,
    _pg: ContainerAsync<PgImage>,
}

impl TestApp {
    pub async fn launch() -> Self {
        let pg = PgImage::default().start().await.unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = db::connect(&url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let (mailer, _outbox) = Mailer::for_test();
        let app = http::router(
            pool.clone(),
            config_for(&url),
            Arc::new(MemoryStorage::new()),
            Arc::new(mailer),
        );
        Self { app, pool, _pg: pg }
    }

    /// POST /api/auth/signup, mark the user verified, then POST /api/auth/login.
    /// Returns (session-cookie, user_id).
    pub async fn signup_with_handle(
        &self,
        display_name: &str,
        handle: &str,
        email: &str,
    ) -> (String, Uuid) {
        let body = serde_json::json!({
            "email": email,
            "password": "verylongpassword",
            "display_name": display_name,
            "handle": handle
        });
        let resp = self
            .app
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
        assert_eq!(resp.status(), 202, "signup must return 202");

        // Look up user_id from DB (handle is normalised lowercase).
        let user_id: Uuid = sqlx::query_scalar!(
            "select id from users where handle = $1",
            handle.to_lowercase()
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();

        // Mark the user verified so that subsequent logins work.
        sqlx::query!(
            "update users set email_verified_at = now() where id = $1",
            user_id
        )
        .execute(&self.pool)
        .await
        .unwrap();

        // Log in to get a session cookie.
        let login_body = serde_json::json!({
            "email": email,
            "password": "verylongpassword"
        });
        let login_resp = self
            .app
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
        let cookie = login_resp
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        (cookie, user_id)
    }

    /// Insert a published, ready photo owned by `user_id`. Returns its UUID.
    /// (Lifted from the pattern in `tests/permalink.rs`.)
    pub async fn ready_photo(&self, user_id: Uuid) -> Uuid {
        self.ready_photo_with(user_id, "ABCD1234", None).await
    }

    pub async fn ready_photo_with(
        &self,
        user_id: Uuid,
        short_id: &str,
        target: Option<&str>,
    ) -> Uuid {
        let photo_id = Uuid::new_v4();
        sqlx::query!(
            "insert into photos \
             (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, target, original_uploaded_at, published_at) \
             values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', $3, $4, now(), now())",
            photo_id,
            user_id,
            short_id,
            target
        )
        .execute(&self.pool)
        .await
        .unwrap();
        photo_id
    }

    pub async fn oneshot(
        &self,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Json>,
    ) -> (StatusCode, Vec<u8>) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let req = if let Some(b) = body {
            req.header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string()))
                .unwrap()
        } else {
            req.body(Body::empty()).unwrap()
        };
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 1_048_576)
            .await
            .unwrap()
            .to_vec();
        (status, bytes)
    }

    pub async fn attach_tags(&self, photo_id: Uuid, tags: &[&str]) {
        let owned: Vec<String> = tags.iter().map(|s| s.to_string()).collect();
        astrophoto::photos::tags::attach(&self.pool, photo_id, &owned)
            .await
            .unwrap();
    }

    pub async fn oneshot_json<T: DeserializeOwned>(
        &self,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Json>,
    ) -> (StatusCode, T) {
        let (status, bytes) = self.oneshot(method, uri, cookie, body).await;
        let parsed: T = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "failed to deserialise body for {} {}: {}\nbody: {}",
                method,
                uri,
                e,
                String::from_utf8_lossy(&bytes)
            )
        });
        (status, parsed)
    }
}
