//! End-to-end upload flow integration test (new init + finalize path).
//!
//! 1. testcontainers Postgres on a random port
//! 2. astrophoto::storage::MemoryStorage in lieu of S3 (no MinIO container needed)
//! 3. signup → POST /api/uploads/init → seed bytes into MemoryStorage →
//!    POST /api/uploads/:id/finalize → GET /api/photos/:id until ready
//! 4. assert GET /api/photos/:id/thumb/400 returns valid JPEG bytes

use std::sync::Arc;

use astrophoto::storage::{MemoryStorage, Storage};
use astrophoto::{Config, db, http};
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
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        storage.clone(),
        Arc::new(mailer),
    );

    // 1. Signup
    let signup_body = serde_json::json!({
        "email": "u@example.com",
        "password": "longenoughpw",
        "display_name": "U",
        "handle": "t-u"
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

    // 2. Init: declare the file to get a photo_id and presigned PUT URL.
    let jpeg_bytes = Bytes::from_static(include_bytes!("fixtures/sample.jpg"));
    let init_body = serde_json::json!({
        "files": [{
            "name": "test.jpg",
            "size": jpeg_bytes.len(),
            "mime": "image/jpeg",
            "hash": "photos-rs-e2e-unique-hash"
        }]
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uploads/init")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(init_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "init should return 200");
    let init_raw = resp.into_body().collect().await.unwrap().to_bytes();
    let init_v: serde_json::Value = serde_json::from_slice(&init_raw).unwrap();
    let photo_id_str = init_v["files"][0]["photo_id"].as_str().unwrap().to_string();
    let photo_id: Uuid = Uuid::parse_str(&photo_id_str).unwrap();

    // 3. Seed bytes into MemoryStorage.
    //    MemoryStorage presigned URLs are stubs; look up the storage_key from the DB.
    let storage_key: String = sqlx::query_scalar!(
        r#"select storage_key as "storage_key!" from photos where id = $1"#,
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    storage
        .put(&storage_key, "image/jpeg", jpeg_bytes)
        .await
        .unwrap();

    // 4. Finalize: trigger the pipeline.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/uploads/{photo_id}/finalize"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "finalize should return 200");
    let fin_raw = resp.into_body().collect().await.unwrap().to_bytes();
    let fin_v: serde_json::Value = serde_json::from_slice(&fin_raw).unwrap();
    assert_eq!(
        fin_v["status"], "ready",
        "finalize must return status=ready"
    );

    // 5. GET /api/photos/:id and confirm status.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/photos/{photo_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let detail_raw = resp.into_body().collect().await.unwrap().to_bytes();
    let detail: serde_json::Value = serde_json::from_slice(&detail_raw).unwrap();
    assert_eq!(detail["status"], "ready");

    // 6. GET /api/photos/:id/thumb/400 returns valid JPEG bytes.
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
    let thumb_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(
        thumb_bytes.starts_with(b"\xff\xd8"),
        "not a JPEG: {:?}",
        &thumb_bytes[..thumb_bytes.len().min(8)]
    );
}
