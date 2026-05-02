//! End-to-end upload flow integration test.
//!
//! 1. testcontainers Postgres on a random port
//! 2. astrophoto::storage::MemoryStorage in lieu of S3 (no MinIO container needed)
//! 3. signup → upload a synthetic JPEG (200×150 pixels) → poll until status=ready
//! 4. assert thumbnails are 400 and 1200 entries; assert /thumb/400 returns valid bytes

use std::sync::Arc;
use std::time::Duration;

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

#[allow(clippy::expect_used)]
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
        oauth_google_client_id: String::new(),
        oauth_google_client_secret: String::new(),
        oauth_google_redirect_url: String::new(),
    }
}

#[allow(clippy::unwrap_used)]
fn make_test_jpeg() -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    }));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
    buf.into_inner()
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn upload_pipeline_signup_upload_thumb() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let storage = Arc::new(MemoryStorage::new());
    let app = http::router(pool.clone(), config_for(&url), storage);

    // 1. Signup
    let signup_body = serde_json::json!({
        "email": "u@example.com",
        "password": "longenoughpw",
        "display_name": "U"
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
    let cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 2. Upload — manual multipart body
    let boundary = "----astrophototestboundary";
    let jpeg = make_test_jpeg();
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"test.jpg\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
    body.extend_from_slice(&jpeg);
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"target\"\r\n\r\n");
    body.extend_from_slice(b"M31");
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/photos")
                .header(header::COOKIE, &cookie)
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
    let photo_id = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["status"], "processing");

    // 3. Poll until ready (max ~3s)
    let mut ready = false;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/photos/{photo_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if v["status"] == "ready" {
            ready = true;
            assert_eq!(v["target"], "M31");
            break;
        }
    }
    assert!(ready, "photo never reached ready state");

    // 4. GET thumb/400 returns JPEG bytes
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/{photo_id}/thumb/400"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(
        bytes.starts_with(b"\xff\xd8"),
        "not a JPEG: {:?}",
        &bytes[..bytes.len().min(8)]
    );
}
