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
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;
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

fn make_test_jpeg() -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    }));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
    buf.into_inner()
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

async fn signup(app: &axum::Router, email: &str, name: &str) -> (String, String) {
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
    assert_eq!(resp.status(), 201);
    let cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = v["id"].as_str().unwrap().to_string();
    (id, cookie)
}

async fn upload(app: &axum::Router, cookie: &str) -> String {
    let boundary = "----testboundary";
    let jpeg = make_test_jpeg();
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"t.jpg\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
    body.extend_from_slice(&jpeg);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/photos")
                .header(header::COOKIE, cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 202);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["id"].as_str().unwrap().to_string()
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
    let (app, _pool, _pg) = boot_app().await;

    let (_owner_id, owner_cookie) = signup(&app, "owner@example.com", "Owner").await;
    let photo_id = upload(&app, &owner_cookie).await;
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

    let (_other_id, cookie) = signup(&app, "u@example.com", "U").await;

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
    let (app, _pool, _pg) = boot_app().await;

    let (_owner_id, owner_cookie) = signup(&app, "owner@example.com", "Owner").await;
    let photo_id = upload(&app, &owner_cookie).await;
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

    let (_b_id, b_cookie) = signup(&app, "b@example.com", "B").await;
    let (_c_id, c_cookie) = signup(&app, "c@example.com", "C").await;

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
    let (app, _pool, _pg) = boot_app().await;

    let (a_id, a_cookie) = signup(&app, "a@example.com", "A").await;
    let (b_id, _b_cookie) = signup(&app, "b@example.com", "B").await;

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
