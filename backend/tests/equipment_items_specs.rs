//! Integration tests for the equipment catalog specs endpoints:
//! GET /api/equipment/items/:id, plus the extended POST and the new
//! PATCH /api/equipment/items/:id added in Tasks 6 and 7.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use tower::ServiceExt;

#[tokio::test]
async fn get_item_returns_specs_when_present() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "carol@example.com", "carol1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('filter','antlia ha 3nm','Antlia Ha 3nm',12,'approved',now(),
                    'Antlia','Ha 3nm')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"insert into filter_specs (item_id, filter_type, bandwidth_nm, size, mounted)
            values ($1,'h_alpha',3.0,'2in',true)"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["kind"], "filter");
    assert_eq!(body["display_name"], "Antlia Ha 3nm");
    assert_eq!(body["specs"]["kind"], "filter");
    assert_eq!(body["specs"]["filter_type"], "h_alpha");
    assert_eq!(body["specs"]["bandwidth_nm"], 3.0);
    assert_eq!(body["specs"]["mounted"], true);
    assert_eq!(body["specs"]["size"], "2in");
}

#[tokio::test]
async fn get_item_returns_null_specs_when_absent() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "dave@example.com", "dave1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('telescope','sw 200p','Sky-Watcher 200P',1,'approved',now(),
                    'Sky-Watcher','200P')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["specs"], serde_json::Value::Null);
}

#[tokio::test]
async fn get_item_returns_404_for_unknown_uuid() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "ed@example.com", "ed1").await;
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{}", uuid::Uuid::new_v4()))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test]
async fn post_item_with_specs_inserts_both_rows() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "frank@example.com", "frank1").await;

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{
                        "kind": "filter",
                        "display_name": "Astrodon 3nm Hα",
                        "specs": {
                            "kind": "filter",
                            "filter_type": "h_alpha",
                            "bandwidth_nm": 3.0,
                            "size": "2in",
                            "mounted": true
                        }
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let id = body["id"].as_str().unwrap();

    // GET the item and verify specs are persisted
    let r2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    let body2: serde_json::Value =
        serde_json::from_slice(&r2.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body2["specs"]["kind"], "filter");
    assert_eq!(body2["specs"]["filter_type"], "h_alpha");
    assert_eq!(body2["specs"]["bandwidth_nm"], 3.0);
    assert_eq!(body2["specs"]["size"], "2in");
    assert_eq!(body2["specs"]["mounted"], true);
}

#[tokio::test]
async fn post_item_wrong_kind_specs_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "grace@example.com", "grace1").await;

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{
                        "kind": "telescope",
                        "display_name": "Mismatched Scope",
                        "specs": {"kind": "filter", "filter_type": "h_alpha"}
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}

#[tokio::test]
async fn post_item_without_specs_still_works() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "heidi@example.com", "heidi1").await;

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"kind":"telescope","display_name":"SW 80ED"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let id = body["id"].as_str().unwrap();

    // GET and confirm specs is null
    let r2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    let body2: serde_json::Value =
        serde_json::from_slice(&r2.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body2["specs"], serde_json::Value::Null);
}

// ── PATCH tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_item_replaces_specs_row() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "ivan@example.com", "ivan1").await;

    // Seed an item with filter_specs (h_alpha, bandwidth_nm=5.0).
    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('filter','antlia oiii 3nm patch test','Antlia OIII 3nm Patch',0,'approved',now(),
                    'Antlia','OIII 3nm Patch')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"insert into filter_specs (item_id, filter_type, bandwidth_nm, size, mounted)
            values ($1,'h_alpha',5.0,'2in',true)"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    // PATCH: replace specs with oiii, bandwidth_nm=3.0.
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"specs":{"kind":"filter","filter_type":"oiii","bandwidth_nm":3.0}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // Re-query filter_specs directly: must be oiii/3.0 with size+mounted wiped.
    let row = sqlx::query!(
        r#"select filter_type, bandwidth_nm, size, mounted
             from filter_specs where item_id = $1"#,
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.filter_type.as_deref(), Some("oiii"));
    assert!(row.size.is_none());
    assert!(row.mounted.is_none());
    let bw: f64 = row.bandwidth_nm.unwrap().to_string().parse().unwrap();
    assert!((bw - 3.0).abs() < 0.01);
}

#[tokio::test]
async fn patch_item_renames_display_name_and_canonical() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "judy@example.com", "judy1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('telescope','old name','Old Name',0,'approved',now(),
                    '','Old Name')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"display_name":" New Name "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    let row = sqlx::query!(
        "select display_name, canonical_name from equipment_items where id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.display_name, "New Name");
    assert_eq!(row.canonical_name, "new name");
}

#[tokio::test]
async fn patch_item_with_wrong_kind_specs_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "karl@example.com", "karl1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('telescope','sw 200p wrong kind','SW 200P Wrong Kind',0,'approved',now(),
                    '','SW 200P Wrong Kind')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // PATCH a telescope item with filter specs — must be 422.
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"specs":{"kind":"filter","filter_type":"h_alpha"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);

    // No filter_specs row must have been created.
    let filter_count: i64 = sqlx::query_scalar!(
        "select count(*) from filter_specs where item_id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(filter_count, 0);

    // No telescope_specs row either (tx aborted before any insert).
    let telescope_count: i64 = sqlx::query_scalar!(
        "select count(*) from telescope_specs where item_id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(telescope_count, 0);
}

#[tokio::test]
async fn patch_item_404_for_unknown_uuid() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "laura@example.com", "laura1").await;

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/items/{}", uuid::Uuid::new_v4()))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"display_name":"Ghost"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test]
async fn patch_item_empty_display_name_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "mike@example.com", "mike1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('mount','celestron avx empty name','Celestron AVX',0,'approved',now(),
                    'Celestron','AVX')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"display_name":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}
