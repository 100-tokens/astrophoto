//! Integration tests for setup CRUD endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{body::Body, http::{Request, header}};
use http_body_util::BodyExt as _;
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
    let router = http::router(pool.clone(), config_for(&url), Arc::new(MemoryStorage::new()), Arc::new(mailer));
    (router, pool)
}

pub async fn signup_and_cookie(app: &axum::Router, email: &str, handle: &str) -> String {
    let body = serde_json::json!({
        "email": email,
        "password": "verylongpassword",
        "display_name": "Test User",
        "handle": handle,
    });
    let resp = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/auth/signup")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), 201, "signup failed");
    resp.headers().get("set-cookie").unwrap().to_str().unwrap().to_string()
}

pub async fn lookup_user_id(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!("select id from users where email = $1", email)
        .fetch_one(pool).await.unwrap()
}

/// Insert a "second user" directly via SQL — useful for cross-user tests
/// where signing up two users via the auth flow is overkill. Inspect
/// `backend/migrations/0001_init.sql` to see the users table schema and
/// the not-null columns; adapt the INSERT below if needed. Returns the
/// user's id.
pub async fn create_other_user(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"insert into users (email, password_hash, display_name, handle)
                values ($1, '$argon2id$v=19$m=19456,t=2,p=1$AAAA$AAAA', 'Other', $2)
           returning id"#,
        email,
        email.split('@').next().unwrap()
    ).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn list_returns_owner_setups_only_with_role_counts() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;

    // Alice has 2 setups: 'Backyard rig' (default, with 2 filters) and 'Travel rig' (no items).
    let s1 = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name, is_default)
         values ($1, 'Backyard rig', true) returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Travel rig')",
        alice_id
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Bob rig')",
        bob_id
    ).execute(&pool).await.unwrap();

    let f1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','ha','Hα',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let f2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','oiii','OIII',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'filter',$2),($1,'filter',$3)",
        s1, f1, f2
    ).execute(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri("/api/equipment/setups")
            .header(header::COOKIE, &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2, "alice has 2 setups; bob's must be excluded");

    let backyard = arr.iter().find(|v| v["name"] == "Backyard rig").unwrap();
    assert_eq!(backyard["is_default"], true);
    let counts = backyard["item_counts"].as_array().unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0]["role"], "filter");
    assert_eq!(counts[0]["count"], 2);

    let travel = arr.iter().find(|v| v["name"] == "Travel rig").unwrap();
    assert_eq!(travel["is_default"], false);
    let travel_counts = travel["item_counts"].as_array().unwrap();
    assert_eq!(travel_counts.len(), 0, "no items → empty array");
}

#[tokio::test]
async fn create_persists_setup_with_items_and_clears_other_default() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;

    sqlx::query!(
        "insert into equipment_setups (owner_id, name, is_default) values ($1,'Old default',true)",
        alice_id
    ).execute(&pool).await.unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind,canonical_name,display_name,usage_count)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();

    let body = serde_json::json!({
        "name": "Backyard rig",
        "description": null,
        "location": "Paris",
        "is_remote": false,
        "is_default": true,
        "guiding": null,
        "items": [{ "role": "optical_tube", "item_id": scope_id.to_string() }]
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/setups")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 201, "expected 201 Created");

    let n_default: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_setups where owner_id=$1 and is_default", alice_id
    ).fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(n_default, 1, "exactly one default per user");

    let backyard_default: bool = sqlx::query_scalar!(
        "select is_default from equipment_setups where owner_id=$1 and name='Backyard rig'",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    assert!(backyard_default, "the new setup is the default");
}

#[tokio::test]
async fn create_unknown_item_id_returns_422() {
    let (app, _pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let body = serde_json::json!({
        "name": "x", "description": null, "location": null,
        "is_remote": false, "is_default": false, "guiding": null,
        "items": [{ "role": "optical_tube",
                    "item_id": "00000000-0000-0000-0000-000000000000" }]
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/setups")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 422);
}

#[tokio::test]
async fn create_duplicate_name_returns_422() {
    let (app, _pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    for expected in [201, 422] {
        let body = serde_json::json!({
            "name": "DupeName", "description": null, "location": null,
            "is_remote": false, "is_default": false, "guiding": null,
            "items": []
        });
        let r = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/setups")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(r.status(), expected);
    }
}

#[tokio::test]
async fn get_one_returns_full_expansion() {
    let (app, pool) = make_app_and_pool().await;
    let cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard rig') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'optical_tube',$2)",
        setup_id, scope_id
    ).execute(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri(&format!("/api/equipment/setups/{setup_id}"))
            .header(header::COOKIE, &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["name"], "Backyard rig");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["role"], "optical_tube");
    assert_eq!(items[0]["item"]["display_name"], "Sky-Watcher 200P");
}

#[tokio::test]
async fn get_one_returns_404_for_other_user() {
    let (app, pool) = make_app_and_pool().await;
    let alice_cookie = signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob rig') returning id",
        bob_id
    ).fetch_one(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri(&format!("/api/equipment/setups/{bob_setup}"))
            .header(header::COOKIE, &alice_cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404);
}
