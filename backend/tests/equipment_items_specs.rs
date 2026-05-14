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
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('filter','antlia ha 3nm','Antlia Ha 3nm',12,'approved',now())
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
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('telescope','sw 200p','Sky-Watcher 200P',1,'approved',now())
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
