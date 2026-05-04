//! Integration tests for apply-setup + detach-setup endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    body::Body,
    http::{Request, header},
};
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

pub async fn make_app_and_pool() -> (axum::Router, sqlx::PgPool) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    std::mem::forget(pg);
    let router = http::router(
        pool.clone(),
        config_for(&url),
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
    );
    (router, pool)
}

pub async fn signup_and_cookie(app: &axum::Router, email: &str, handle: &str) -> String {
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

pub async fn lookup_user_id(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!("select id from users where email = $1", email)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Insert a "second user" directly via SQL — useful for cross-user tests
/// where signing up two users via the auth flow is overkill.
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

/// Insert a minimal photo row. Accepts scope and camera so apply-setup
/// tests can seed pre-filled EXIF-style columns.
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

#[tokio::test]
async fn fill_empty_preserves_existing_camera_and_fills_missing_columns() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;

    // EXIF already filled `camera` to "Canon EOS 6D"; nothing else.
    let photo_id = insert_stub_photo(&pool, alice_id, None, None, Some("Canon EOS 6D".into())).await;

    // Setup with main_camera = ZWO ASI2600, optical_tube = SW 200P.
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let cam_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi2600','ZWO ASI2600',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope','sw 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'main_camera',$2),($1,'optical_tube',$3)",
        setup_id, cam_id, scope_id
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "fill_empty" });
    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{photo_id}/apply-setup"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);

    let row = sqlx::query!(
        "select setup_id, scope, camera from photos where id=$1", photo_id
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.setup_id, Some(setup_id), "setup_id is set even though camera unchanged");
    assert_eq!(row.scope.as_deref(), Some("Sky-Watcher 200P"), "empty scope filled from setup");
    assert_eq!(row.camera.as_deref(), Some("Canon EOS 6D"), "EXIF camera preserved");
}

#[tokio::test]
async fn overwrite_writes_all_columns_verbatim() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let photo_id = insert_stub_photo(&pool, alice_id, None, Some("Some scope".into()), Some("Canon EOS 6D".into())).await;

    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let cam_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi2600','ZWO ASI2600',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'main_camera',$2)",
        setup_id, cam_id
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "overwrite" });
    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{photo_id}/apply-setup"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let row = sqlx::query!("select scope, camera from photos where id=$1", photo_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.scope, None, "no optical_tube → scope cleared in overwrite");
    assert_eq!(row.camera.as_deref(), Some("ZWO ASI2600"), "EXIF camera replaced by setup");
}

#[tokio::test]
async fn multiple_filters_join_alphabetical() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let photo_id = insert_stub_photo(&pool, alice_id, None, None, None).await;

    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Mono SHO') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let f1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','sii','SII',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let f2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','ha','Hα',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let f3 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','oiii','OIII',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'filter',$2),($1,'filter',$3),($1,'filter',$4)",
        setup_id, f1, f2, f3
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "overwrite" });
    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{photo_id}/apply-setup"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let row = sqlx::query!("select filters from photos where id=$1", photo_id)
        .fetch_one(&pool).await.unwrap();
    // canonical_name order: 'ha', 'oiii', 'sii' → alphabetical ASCII.
    assert_eq!(row.filters.as_deref(), Some("Hα, OIII, SII"));
}

#[tokio::test]
async fn detach_clears_setup_id_only() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'X') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let photo_id = insert_stub_photo(&pool, alice_id, Some(setup_id), Some("Sky-Watcher 200P".into()), None).await;

    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{photo_id}/detach-setup"))
            .header(header::COOKIE, &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 204);
    let row = sqlx::query!("select setup_id, scope from photos where id=$1", photo_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.setup_id, None);
    assert_eq!(row.scope.as_deref(), Some("Sky-Watcher 200P"), "denorm columns untouched");
}

#[tokio::test]
async fn cross_user_photo_or_setup_returns_404() {
    let (app, pool) = make_app_and_pool().await;
    let alice_cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;

    // Alice's photo, Bob's setup.
    let alice_photo = insert_stub_photo(&pool, alice_id, None, None, None).await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    ).fetch_one(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": bob_setup.to_string(), "mode": "overwrite" });
    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{alice_photo}/apply-setup"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &alice_cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404, "bob's setup not visible to alice");

    // Alice's setup, Bob's photo.
    let alice_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Alice') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let bob_photo = insert_stub_photo(&pool, bob_id, None, None, None).await;
    let body = serde_json::json!({ "setup_id": alice_setup.to_string(), "mode": "overwrite" });
    let r = app.clone().oneshot(
        Request::builder().method("POST")
            .uri(&format!("/api/photos/{bob_photo}/apply-setup"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &alice_cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404, "alice can't apply to bob's photo");
}
