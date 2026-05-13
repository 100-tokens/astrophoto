//! Integration tests for the engagement layer (Phase 7):
//! appreciations, comments, follows.
//!
//! Uses an ephemeral Postgres via testcontainers and an in-memory
//! Storage so the upload pipeline works without MinIO.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use astrophoto::storage::MemoryStorage;
use astrophoto::{Config, db, http};
use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use sqlx::PgPool;
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

fn handle_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("user");
    let mut h = local
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    h = h.trim_matches('-').to_string();
    if h.len() < 3 {
        h = format!("t-{h}");
    }
    h
}

async fn signup(app: &axum::Router, pool: &PgPool, email: &str, name: &str) -> (String, String) {
    let handle = handle_from_email(email);
    let body = serde_json::json!({
        "email": email, "password": "longenoughpw", "display_name": name, "handle": handle,
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
    assert_eq!(resp.status(), 202);

    // Mark user verified so login works.
    sqlx::query!(
        "update users set email_verified_at = now() where email = $1",
        email
    )
    .execute(pool)
    .await
    .unwrap();

    // Log in to obtain a session cookie.
    let login_body = serde_json::json!({"email": email, "password": "longenoughpw"});
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
    assert_eq!(login_resp.status(), 200, "login must succeed after signup for {email}");
    let cookie = login_resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let bytes = login_resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = v["id"].as_str().unwrap().to_string();
    (id, cookie)
}

/// Seed a ready photo for the authenticated user (identified by cookie)
/// using direct SQL, bypassing the upload pipeline.
/// Returns the photo id as a String.
async fn upload(pool: &PgPool, app: &axum::Router, cookie: &str) -> String {
    // Resolve the user id from the session.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let owner_id: uuid::Uuid = uuid::Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();

    let photo_id = uuid::Uuid::new_v4();
    let storage_key = format!("originals/test-{photo_id}");
    let short_id = format!("E{}", &photo_id.to_string().replace('-', "")[..7]);
    sqlx::query!(
        r#"
        insert into photos
            (id, owner_id, storage_key, original_name, bytes, mime,
             short_id, status, last_step, original_uploaded_at)
        values ($1, $2, $3, 'test.jpg', 1000, 'image/jpeg',
                $4, 'ready', 'upload', now())
        "#,
        photo_id,
        owner_id,
        storage_key,
        short_id,
    )
    .execute(pool)
    .await
    .unwrap();

    photo_id.to_string()
}

async fn boot_app() -> (
    axum::Router,
    sqlx::PgPool,
    testcontainers::ContainerAsync<PgImage>,
) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(pool.clone(), config_for(&url), storage, Arc::new(mailer));
    (app, pool, pg)
}

async fn publish_photo(app: &axum::Router, cookie: &str, id: &str) {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{id}/publish"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "publish_photo failed for {id}");
}

async fn json_get(app: &axum::Router, uri: &str, cookie: Option<&str>) -> serde_json::Value {
    let mut req = Request::builder().uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let resp = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if !status.is_success() {
        panic!(
            "GET {uri} failed: {status} {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn appreciation_toggle() {
    let (app, pool, _pg) = boot_app().await;

    let (_owner_id, owner_cookie) = signup(&app, &pool, "owner@example.com", "Owner").await;
    let photo_id = upload(&pool, &app, &owner_cookie).await;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let v = json_get(
            &app,
            &format!("/api/photos/{photo_id}"),
            Some(&owner_cookie),
        )
        .await;
        if v["status"] == "ready" {
            break;
        }
    }
    publish_photo(&app, &owner_cookie, &photo_id).await;

    let (_other_id, cookie) = signup(&app, &pool, "u@example.com", "U").await;

    let v = json_get(
        &app,
        &format!("/api/photos/{photo_id}/appreciations/count"),
        None,
    )
    .await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);

    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/photos/{photo_id}/appreciate"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    let v = json_get(
        &app,
        &format!("/api/photos/{photo_id}/appreciations/count"),
        None,
    )
    .await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);

    let v = json_get(
        &app,
        &format!("/api/photos/{photo_id}/appreciation-state"),
        Some(&cookie),
    )
    .await;
    assert!(v["appreciated"].as_bool().unwrap());

    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/photos/{photo_id}/appreciate"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    let v = json_get(
        &app,
        &format!("/api/photos/{photo_id}/appreciations/count"),
        None,
    )
    .await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn comment_create_list_delete_authorization() {
    let (app, pool, _pg) = boot_app().await;

    let (_owner_id, owner_cookie) = signup(&app, &pool, "owner@example.com", "Owner").await;
    let photo_id = upload(&pool, &app, &owner_cookie).await;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let v = json_get(
            &app,
            &format!("/api/photos/{photo_id}"),
            Some(&owner_cookie),
        )
        .await;
        if v["status"] == "ready" {
            break;
        }
    }
    publish_photo(&app, &owner_cookie, &photo_id).await;

    let (_b_id, b_cookie) = signup(&app, &pool, "b@example.com", "B").await;
    let (_c_id, c_cookie) = signup(&app, &pool, "c@example.com", "C").await;

    let body = serde_json::json!({ "body": "Looks great!" });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/comments"))
                .header(header::COOKIE, &b_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let comment_id = v["id"].as_str().unwrap().to_string();

    let v = json_get(&app, &format!("/api/photos/{photo_id}/comments"), None).await;
    assert_eq!(v["comments"].as_array().unwrap().len(), 1);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/comments/{comment_id}"))
                .header(header::COOKIE, &c_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/comments/{comment_id}"))
                .header(header::COOKIE, &owner_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let v = json_get(&app, &format!("/api/photos/{photo_id}/comments"), None).await;
    assert_eq!(v["comments"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn follow_toggle_and_counts() {
    let (app, pool, _pg) = boot_app().await;

    let (a_id, a_cookie) = signup(&app, &pool, "a@example.com", "A").await;
    let (b_id, _b_cookie) = signup(&app, &pool, "b@example.com", "B").await;

    let v = json_get(&app, &format!("/api/users/{a_id}/following/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);

    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{b_id}/follow"))
                    .header(header::COOKIE, &a_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    let v = json_get(&app, &format!("/api/users/{a_id}/following/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);
    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);

    let v = json_get(&app, "/api/auth/me", Some(&a_cookie)).await;
    let following = v["following_ids"].as_array().unwrap();
    assert_eq!(following.len(), 1);
    assert_eq!(following[0].as_str().unwrap(), b_id);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/users/{a_id}/follow"))
                .header(header::COOKIE, &a_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/users/{b_id}/follow"))
                .header(header::COOKIE, &a_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
}
