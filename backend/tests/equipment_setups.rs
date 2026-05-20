//! Integration tests for setup CRUD endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn list_returns_owner_setups_only_with_role_counts() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let bob_id = common::create_other_user(&pool, "bob@example.com").await;

    // Alice has 2 setups: 'Backyard rig' (default, with 2 filters) and 'Travel rig' (no items).
    let s1 = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name, is_default)
         values ($1, 'Backyard rig', true) returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Travel rig')",
        alice_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Bob rig')",
        bob_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let f1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('filter','ha','Hα',0,'','Hα') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let f2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('filter','oiii','OIII',0,'','OIII') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'filter',$2),($1,'filter',$3)",
        s1, f1, f2
    ).execute(&pool).await.unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/equipment/setups")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
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
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;

    sqlx::query!(
        "insert into equipment_setups (owner_id, name, is_default) values ($1,'Old default',true)",
        alice_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind,canonical_name,display_name,usage_count,brand,model)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0,'Sky-Watcher','200P') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "name": "Backyard rig",
        "description": null,
        "location": "Paris",
        "is_remote": false,
        "is_default": true,
        "guiding": null,
        "default_apply_mode": "overwrite",
        "items": [{ "role": "optical_tube", "item_id": scope_id.to_string() }]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/setups")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 201, "expected 201 Created");

    let n_default: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_setups where owner_id=$1 and is_default",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(n_default, 1, "exactly one default per user");

    let backyard_default: bool = sqlx::query_scalar!(
        "select is_default from equipment_setups where owner_id=$1 and name='Backyard rig'",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(backyard_default, "the new setup is the default");
}

#[tokio::test]
async fn create_unknown_item_id_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let body = serde_json::json!({
        "name": "x", "description": null, "location": null,
        "is_remote": false, "is_default": false, "guiding": null,
        "default_apply_mode": "overwrite",
        "items": [{ "role": "optical_tube",
                    "item_id": "00000000-0000-0000-0000-000000000000" }]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/setups")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}

#[tokio::test]
async fn create_duplicate_name_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    for expected in [201, 422] {
        let body = serde_json::json!({
            "name": "DupeName", "description": null, "location": null,
            "is_remote": false, "is_default": false, "guiding": null,
            "default_apply_mode": "overwrite",
            "items": []
        });
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/equipment/setups")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
    }
}

#[tokio::test]
async fn get_one_returns_full_expansion() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard rig') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0,'Sky-Watcher','200P') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'optical_tube',$2)",
        setup_id,
        scope_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/setups/{setup_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
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
    let (app, pool) = common::make_app_and_pool().await;
    let alice_cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let bob_id = common::create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob rig') returning id",
        bob_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/setups/{bob_setup}"))
                .header(header::COOKIE, &alice_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test]
async fn update_replaces_items_and_meta() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name, location)
         values ($1,'Backyard','Paris') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let i1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0,'Sky-Watcher','200P') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let i2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count, brand, model)
         values ('camera','asi2600','ZWO ASI2600',0,'ZWO','ASI2600') returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'optical_tube',$2)",
        setup_id,
        i1
    )
    .execute(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "name": "Backyard rig v2",
        "description": null,
        "location": "Paris",
        "is_remote": false,
        "is_default": false,
        "guiding": null,
        "default_apply_mode": "overwrite",
        "items": [
            { "role": "optical_tube", "item_id": i1.to_string() },
            { "role": "main_camera",  "item_id": i2.to_string() }
        ]
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/setups/{setup_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    let n_items: i64 = sqlx::query_scalar!(
        "select count(*) from setup_items where setup_id=$1",
        setup_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(n_items, 2);
    let new_name: String =
        sqlx::query_scalar!("select name from equipment_setups where id=$1", setup_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(new_name, "Backyard rig v2");
}

#[tokio::test]
async fn update_promote_to_default_clears_previous() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    sqlx::query!(
        "insert into equipment_setups (owner_id, name, is_default) values ($1,'Old',true)",
        alice_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let new_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'New') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "name": "New", "description": null, "location": null,
        "is_remote": false, "is_default": true, "guiding": null,
        "default_apply_mode": "overwrite",
        "items": []
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/setups/{new_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let n_default: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_setups where owner_id=$1 and is_default",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(n_default, 1);
}

#[tokio::test]
async fn update_returns_404_for_other_user() {
    let (app, pool) = common::make_app_and_pool().await;
    let alice_cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let bob_id = common::create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let body = serde_json::json!({
        "name": "Hacked", "description": null, "location": null,
        "is_remote": false, "is_default": false, "guiding": null,
        "default_apply_mode": "overwrite", "items": []
    });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/setups/{bob_setup}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &alice_cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test]
async fn delete_clears_photos_setup_id_but_keeps_denorm_columns() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'X') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let photo_id = common::insert_stub_photo(
        &pool,
        alice_id,
        Some(setup_id),
        Some("Sky-Watcher 200P".into()),
        None,
    )
    .await;

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/equipment/setups/{setup_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 204);

    let row = sqlx::query!("select setup_id, scope from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.setup_id, None);
    assert_eq!(row.scope.as_deref(), Some("Sky-Watcher 200P"));
}

#[tokio::test]
async fn delete_returns_404_for_other_user() {
    let (app, pool) = common::make_app_and_pool().await;
    let alice_cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;
    let bob_id = common::create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/equipment/setups/{bob_setup}"))
                .header(header::COOKIE, &alice_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

// ─── default_apply_mode (migration 0019) ────────────────────────────────────

/// Helper: build a minimal SetupInput body, optionally overriding
/// `default_apply_mode`. Keeps the four tests below readable.
fn setup_body(name: &str, default_apply_mode: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": null,
        "location": null,
        "is_remote": false,
        "is_default": false,
        "guiding": null,
        "default_apply_mode": default_apply_mode,
        "items": []
    })
}

async fn post_setup(
    app: &axum::Router,
    cookie: &str,
    body: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/equipment/setups")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, parsed)
}

#[tokio::test]
async fn create_persists_overwrite_default_apply_mode() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;

    let (status, detail) = post_setup(&app, &cookie, setup_body("default_mode", "overwrite")).await;
    assert_eq!(status, 201);
    assert_eq!(detail["default_apply_mode"], "overwrite");

    // Roundtrip via GET — confirm it persists.
    let id = detail["id"].as_str().unwrap();
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/setups/{id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let got: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(got["default_apply_mode"], "overwrite");
}

#[tokio::test]
async fn create_persists_fill_empty_default_apply_mode_and_roundtrips() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;

    let (status, detail) = post_setup(&app, &cookie, setup_body("fill_only", "fill_empty")).await;
    assert_eq!(status, 201);
    assert_eq!(detail["default_apply_mode"], "fill_empty");

    let id = detail["id"].as_str().unwrap();
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/setups/{id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let got: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(got["default_apply_mode"], "fill_empty");

    // Also confirm the list view surfaces the persisted value.
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/equipment/setups")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let arr: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let row = arr
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
        .unwrap();
    assert_eq!(row["default_apply_mode"], "fill_empty");
}

#[tokio::test]
async fn create_invalid_default_apply_mode_returns_422() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;

    let (status, _) = post_setup(&app, &cookie, setup_body("bad_mode", "banana")).await;
    assert_eq!(status, 422);
}

#[tokio::test]
async fn update_persists_changed_default_apply_mode() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "alice@example.com", "alice1").await;

    // Create with the default 'overwrite'.
    let (status, detail) = post_setup(&app, &cookie, setup_body("toggle", "overwrite")).await;
    assert_eq!(status, 201);
    let id = detail["id"].as_str().unwrap();

    // PATCH to 'fill_empty'.
    let mut body = setup_body("toggle", "fill_empty");
    // Update preserves the name; ensure it matches what we created.
    body["name"] = serde_json::Value::String("toggle".into());
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/setups/{id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let after: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(after["default_apply_mode"], "fill_empty");

    // PATCH with an invalid value must 422 and not mutate the row.
    let bad = setup_body("toggle", "neither");
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/equipment/setups/{id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(bad.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    let stored: String = sqlx::query_scalar!(
        "select default_apply_mode from equipment_setups where id = $1",
        Uuid::parse_str(id).unwrap()
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, "fill_empty");
}
