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

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

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

    /// POST /api/auth/signup with the given handle. Returns (cookie, user_id).
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
        assert_eq!(resp.status(), 201, "signup must succeed");
        let cookie = resp
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        // Look up user_id from DB (handle is normalised lowercase).
        let user_id: Uuid = sqlx::query_scalar!(
            "select id from users where handle = $1",
            handle.to_lowercase()
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();
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
        let bytes = to_bytes(resp.into_body(), 1_048_576).await.unwrap().to_vec();
        (status, bytes)
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
