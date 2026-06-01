//! Integration tests for the super-admin surface (`/api/admin/*`): the
//! `AdminUser` guard, settings read/write + the signup gate, and equipment
//! list/edit/delete.

mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

#[allow(clippy::unwrap_used)]
async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap().to_vec();
    (status, bytes)
}

#[allow(clippy::unwrap_used)]
async fn mark_admin(pool: &sqlx::PgPool, email: &str) {
    sqlx::query!("update users set is_admin = true where email = $1", email)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn admin_settings_requires_admin() {
    let (app, pool) = common::make_app_and_pool().await;

    // Anonymous → 401.
    let (status, _) = send(&app, "GET", "/api/admin/settings", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Authenticated non-admin → 403.
    let cookie = common::signup_and_cookie(&app, &pool, "plain@example.com", "plainuser").await;
    let (status, _) = send(&app, "GET", "/api/admin/settings", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Promote the same user → the existing session is now an admin (role is
    // read per-request, not baked into the session).
    mark_admin(&pool, "plain@example.com").await;
    let (status, body) = send(&app, "GET", "/api/admin/settings", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let s: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(s["signups_enabled"], true);
    assert_eq!(s["free_upload_max_mb"], 50);
    assert_eq!(s["subscriber_upload_max_mb"], 200);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn settings_update_gates_signup() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "boss@example.com", "boss").await;
    mark_admin(&pool, "boss@example.com").await;

    // Disable signups + change limits.
    let (status, body) = send(
        &app,
        "PUT",
        "/api/admin/settings",
        Some(&cookie),
        Some(json!({
            "signups_enabled": false,
            "free_upload_max_mb": 25,
            "subscriber_upload_max_mb": 100
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let s: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(s["free_upload_max_mb"], 25);

    // A new signup is now refused (400).
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(json!({
            "email": "blocked@example.com",
            "password": "verylongpassword",
            "display_name": "Blocked",
            "handle": "blocked"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Re-enable → signup works again (202).
    let (status, _) = send(
        &app,
        "PUT",
        "/api/admin/settings",
        Some(&cookie),
        Some(json!({
            "signups_enabled": true,
            "free_upload_max_mb": 25,
            "subscriber_upload_max_mb": 100
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(json!({
            "email": "ok@example.com",
            "password": "verylongpassword",
            "display_name": "Ok",
            "handle": "okuser"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn settings_put_rejects_non_admin() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "sneaky@example.com", "sneaky").await;
    let (status, _) = send(
        &app,
        "PUT",
        "/api/admin/settings",
        Some(&cookie),
        Some(json!({"signups_enabled": false, "free_upload_max_mb": 1, "subscriber_upload_max_mb": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_admin_list_edit_delete() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "curator@example.com", "curator").await;
    mark_admin(&pool, "curator@example.com").await;

    let item_id = Uuid::new_v4();
    sqlx::query!(
        r#"insert into equipment_items
              (id, kind, canonical_name, display_name, brand, model, status)
           values ($1, 'camera', 'zwo asi533', 'ZWO ASI533', 'ZWO', 'ASI533', 'approved')"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    // List (filtered by kind) includes the item.
    let (status, body) = send(
        &app,
        "GET",
        "/api/admin/equipment?kind=camera",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page: Value = serde_json::from_slice(&body).unwrap();
    assert!(
        page["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == item_id.to_string()),
        "listing should include the seeded item"
    );

    // Edit a structured field (model) → display_name + canonical regenerated
    // from brand/model/variant.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        Some(json!({ "model": "ASI533 MC" })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let row = sqlx::query!(
        "select display_name, canonical_name from equipment_items where id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.display_name, "ZWO ASI533 MC");
    assert_eq!(row.canonical_name, "zwo asi533 mc");

    // Delete an orphaned item → 204, gone.
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let count: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from equipment_items where id = $1"#,
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_delete_refuses_in_use_item() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "mod@example.com", "moder").await;
    mark_admin(&pool, "mod@example.com").await;

    let item_id = Uuid::new_v4();
    sqlx::query!(
        r#"insert into equipment_items
              (id, kind, canonical_name, display_name, brand, model, status, usage_count)
           values ($1, 'telescope', 'sky-watcher esprit 100', 'Sky-Watcher Esprit 100', 'Sky-Watcher', 'Esprit 100', 'approved', 3)"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "in-use item must not be deletable"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn equipment_admin_edit_specs_and_status_lossless() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "specs@example.com", "specsadmin").await;
    mark_admin(&pool, "specs@example.com").await;

    let item_id = Uuid::new_v4();
    sqlx::query!(
        r#"insert into equipment_items
              (id, kind, canonical_name, display_name, brand, model, status)
           values ($1, 'camera', 'zwo asi2600', 'ZWO ASI2600', 'ZWO', 'ASI2600', 'approved')"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let specs = json!({
        "kind": "camera", "sensor_type": "cmos", "cooled": true,
        "read_noise_e": 1.5, "full_well_capacity_e": 51000, "mount_thread": "M42"
    });

    // Set per-kind specs (incl. v2 completeness fields) + moderation status.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        Some(json!({ "status": "pending", "specs": specs })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = send(
        &app,
        "GET",
        &format!("/api/equipment/items/{item_id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let detail: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail["status"], "pending");
    assert_eq!(
        detail["specs"]["read_noise_e"], 1.5,
        "v2 spec field persisted"
    );
    assert_eq!(detail["specs"]["mount_thread"], "M42");
    let specs_a = detail["specs"].clone();

    // Round-trip: re-submit the read-back specs (a no-op admin save) — the
    // stored specs must be byte-for-byte identical (no v2 field dropped).
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        Some(json!({ "specs": specs_a })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body2) = send(
        &app,
        "GET",
        &format!("/api/equipment/items/{item_id}"),
        Some(&cookie),
        None,
    )
    .await;
    let detail2: Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(detail2["specs"], specs_a, "lossless specs round-trip");

    // `merged` status is not admin-settable.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/admin/equipment/{item_id}"),
        Some(&cookie),
        Some(json!({ "status": "merged" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
