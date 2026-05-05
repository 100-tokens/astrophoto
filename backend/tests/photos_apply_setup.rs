//! Integration tests for apply-setup + detach-setup endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use tower::ServiceExt;

#[tokio::test]
async fn fill_empty_preserves_existing_camera_and_fills_missing_columns() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;

    // EXIF already filled `camera` to "Canon EOS 6D"; nothing else.
    let photo_id =
        common::insert_stub_photo(&pool, alice_id, None, None, Some("Canon EOS 6D".into())).await;

    // Setup with main_camera = ZWO ASI2600, optical_tube = SW 200P.
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let cam_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi2600','ZWO ASI2600',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope','sw 200p','Sky-Watcher 200P',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'main_camera',$2),($1,'optical_tube',$3)",
        setup_id, cam_id, scope_id
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "fill_empty" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/apply-setup"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    let row = sqlx::query!(
        "select setup_id, scope, camera from photos where id=$1",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row.setup_id,
        Some(setup_id),
        "setup_id is set even though camera unchanged"
    );
    assert_eq!(
        row.scope.as_deref(),
        Some("Sky-Watcher 200P"),
        "empty scope filled from setup"
    );
    assert_eq!(
        row.camera.as_deref(),
        Some("Canon EOS 6D"),
        "EXIF camera preserved"
    );
}

#[tokio::test]
async fn overwrite_writes_all_columns_verbatim() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let photo_id = common::insert_stub_photo(
        &pool,
        alice_id,
        None,
        Some("Some scope".into()),
        Some("Canon EOS 6D".into()),
    )
    .await;

    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let cam_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi2600','ZWO ASI2600',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'main_camera',$2)",
        setup_id,
        cam_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "overwrite" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/apply-setup"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let row = sqlx::query!("select scope, camera from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        row.scope, None,
        "no optical_tube → scope cleared in overwrite"
    );
    assert_eq!(
        row.camera.as_deref(),
        Some("ZWO ASI2600"),
        "EXIF camera replaced by setup"
    );
}

#[tokio::test]
async fn multiple_filters_join_alphabetical() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let photo_id = common::insert_stub_photo(&pool, alice_id, None, None, None).await;

    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Mono SHO') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let f1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','sii','SII',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let f2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','ha','Hα',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let f3 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','oiii','OIII',0) returning id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'filter',$2),($1,'filter',$3),($1,'filter',$4)",
        setup_id, f1, f2, f3
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({ "setup_id": setup_id.to_string(), "mode": "overwrite" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/apply-setup"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let row = sqlx::query!("select filters from photos where id=$1", photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    // canonical_name order: 'ha', 'oiii', 'sii' → alphabetical ASCII.
    assert_eq!(row.filters.as_deref(), Some("Hα, OIII, SII"));
}

#[tokio::test]
async fn detach_clears_setup_id_only() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
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
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/detach-setup"))
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
    assert_eq!(
        row.scope.as_deref(),
        Some("Sky-Watcher 200P"),
        "denorm columns untouched"
    );
}

#[tokio::test]
async fn cross_user_photo_or_setup_returns_404() {
    let (app, pool) = common::make_app_and_pool().await;
    let alice_cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;
    let bob_id = common::create_other_user(&pool, "bob@example.com").await;

    // Alice's photo, Bob's setup.
    let alice_photo = common::insert_stub_photo(&pool, alice_id, None, None, None).await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({ "setup_id": bob_setup.to_string(), "mode": "overwrite" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{alice_photo}/apply-setup"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &alice_cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404, "bob's setup not visible to alice");

    // Alice's setup, Bob's photo.
    let alice_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Alice') returning id",
        alice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let bob_photo = common::insert_stub_photo(&pool, bob_id, None, None, None).await;
    let body = serde_json::json!({ "setup_id": alice_setup.to_string(), "mode": "overwrite" });
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{bob_photo}/apply-setup"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &alice_cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 404, "alice can't apply to bob's photo");
}

#[tokio::test]
async fn apply_replaces_existing_setup_id_and_columns_in_overwrite_mode() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, "alice@example.com", "alice1").await;
    let alice_id = common::lookup_user_id(&pool, "alice@example.com").await;

    let setup_a = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'A') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let setup_b = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'B') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let cam_b = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi6200','ZWO ASI6200',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'main_camera',$2)",
        setup_b, cam_b
    ).execute(&pool).await.unwrap();

    // Photo currently references setup_a with camera='Canon EOS 6D'.
    let photo_id = common::insert_stub_photo(
        &pool, alice_id, Some(setup_a), None, Some("Canon EOS 6D".into())
    ).await;

    // Apply setup_b in overwrite mode.
    let body = serde_json::json!({ "setup_id": setup_b.to_string(), "mode": "overwrite" });
    let r = app.clone().oneshot(
        axum::http::Request::builder().method("POST")
            .uri(format!("/api/photos/{photo_id}/apply-setup"))
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .header(axum::http::header::COOKIE, &cookie)
            .body(axum::body::Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);

    let row = sqlx::query!("select setup_id, camera from photos where id=$1", photo_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.setup_id, Some(setup_b), "setup_id replaced");
    assert_eq!(row.camera.as_deref(), Some("ZWO ASI6200"), "camera replaced verbatim");
}
