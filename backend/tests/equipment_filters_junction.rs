//! PUT /api/photos/:id structured filters branch: junction sync,
//! cache rebuild, validation, and the legacy/structured precedence rule.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use tower::ServiceExt;

#[tokio::test]
async fn put_photo_filter_item_ids_writes_junction_and_rebuilds_cache() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie =
        common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let photo_id = common::insert_stub_photo(&pool, alice_id, None, None, None).await;

    // Insert 3 filter items: Ha, OIII, SII
    let ha_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('filter','antlia ha','Antlia Hα',0,'approved',now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let oiii_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('filter','antlia oiii','Antlia OIII',0,'approved',now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let sii_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('filter','antlia sii','Antlia SII',0,'approved',now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "filter_item_ids": [ha_id.to_string(), oiii_id.to_string(), sii_id.to_string()]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/photos/{photo_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "expected 200 from PUT /api/photos/:id");

    // Verify the junction has 3 rows at positions 0, 1, 2.
    let pairs: Vec<(uuid::Uuid, i16)> = sqlx::query!(
        "select item_id, position from photo_filters where photo_id=$1 order by position",
        photo_id
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.item_id, r.position))
    .collect();

    assert_eq!(pairs.len(), 3, "expected 3 junction rows");
    assert_eq!(pairs[0], (ha_id, 0));
    assert_eq!(pairs[1], (oiii_id, 1));
    assert_eq!(pairs[2], (sii_id, 2));

    // Verify the cache string is rebuilt from the junction.
    let filters_cache = sqlx::query_scalar!("select filters from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        filters_cache.as_deref(),
        Some("Antlia Hα, Antlia OIII, Antlia SII"),
        "cache must reflect junction order (positions 0,1,2)"
    );
}

#[tokio::test]
async fn put_photo_filter_item_ids_with_non_filter_kind_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie =
        common::signup_and_cookie(&app, &pool, "bob@example.com", "bob1").await;
    let bob_id = common::lookup_user_id(&pool, "bob@example.com").await;
    let photo_id = common::insert_stub_photo(&pool, bob_id, None, None, None).await;

    // Insert a telescope item (not a filter).
    let telescope_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('telescope','sw 200p','Sky-Watcher 200P',0,'approved',now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "filter_item_ids": [telescope_id.to_string()]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/photos/{photo_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422, "non-filter kind must return 422");

    // Junction must still be empty (tx rolled back).
    let count: i64 = sqlx::query_scalar!(
        "select count(*) from photo_filters where photo_id=$1",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 0, "junction must remain empty after validation failure");
}

#[tokio::test]
async fn put_photo_legacy_filters_text_still_accepted() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie =
        common::signup_and_cookie(&app, &pool, "carol@example.com", "carol1").await;
    let carol_id = common::lookup_user_id(&pool, "carol@example.com").await;
    let photo_id = common::insert_stub_photo(&pool, carol_id, None, None, None).await;

    let body = serde_json::json!({ "filters": "L, R, G" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/photos/{photo_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "legacy filters text must be accepted");

    // photos.filters must hold the literal text.
    let filters_cache = sqlx::query_scalar!("select filters from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        filters_cache.as_deref(),
        Some("L, R, G"),
        "cache must hold the legacy freetext"
    );

    // Junction must be empty — legacy path doesn't auto-populate it.
    let count: i64 = sqlx::query_scalar!(
        "select count(*) from photo_filters where photo_id=$1",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 0, "legacy text path must not touch the junction");
}

#[tokio::test]
async fn put_photo_structured_wins_over_legacy_text_when_both_present() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie =
        common::signup_and_cookie(&app, &pool, "dave@example.com", "dave1").await;
    let dave_id = common::lookup_user_id(&pool, "dave@example.com").await;
    let photo_id = common::insert_stub_photo(&pool, dave_id, None, None, None).await;

    // Insert 1 filter item.
    let x_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at)
            values ('filter','x-filter','X-filter',0,'approved',now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Send both filter_item_ids and legacy filters text.
    let body = serde_json::json!({
        "filter_item_ids": [x_id.to_string()],
        "filters": "ignored"
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/photos/{photo_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "structured + legacy both present must succeed");

    // Cache must reflect the junction (X-filter), NOT the legacy text "ignored".
    let filters_cache = sqlx::query_scalar!("select filters from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        filters_cache.as_deref(),
        Some("X-filter"),
        "structured wins: cache must be rebuilt from junction, not legacy text"
    );

    // Junction must have exactly 1 row.
    let pairs: Vec<(uuid::Uuid, i16)> = sqlx::query!(
        "select item_id, position from photo_filters where photo_id=$1 order by position",
        photo_id
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.item_id, r.position))
    .collect();

    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (x_id, 0));
}
