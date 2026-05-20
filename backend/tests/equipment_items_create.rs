//! Integration tests for Task 4: POST /api/equipment/items resolve-or-create
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use tower::ServiceExt;

#[tokio::test]
async fn insert_on_miss_returns_row_with_zero_count() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"kind":"telescope","display_name":"Sky-Watcher 200P"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["display_name"], "Sky-Watcher 200P");
    assert_eq!(body["canonical_name"], "sky-watcher 200p");
    assert_eq!(body["kind"], "telescope");
    let row = sqlx::query!(
        "select usage_count, status, approved_at
           from equipment_items
          where kind='telescope' and canonical_name='sky-watcher 200p'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.usage_count, 0);
    assert_eq!(row.status, "approved");
    assert!(
        row.approved_at.is_some(),
        "auto-approved items must carry an approved_at timestamp"
    );
}

#[tokio::test]
async fn idempotent_on_hit_does_not_increment() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "bob@example.com", "bob1").await;
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('telescope', 'celestron c8', 'Celestron C8', 7, 'Celestron', 'C8')"
    )
    .execute(&pool)
    .await
    .unwrap();
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"kind":"telescope","display_name":"Celestron C8"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let count: i32 = sqlx::query_scalar!(
        "select usage_count from equipment_items where kind='telescope' and canonical_name='celestron c8'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 7);
}

#[tokio::test]
async fn invalid_kind_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "carol@example.com", "carol1").await;
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"kind":"banana","display_name":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}
