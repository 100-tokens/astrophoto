mod common;

use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

async fn pin(app: &TestApp, cookie: &str, photo_id: Uuid) {
    app.oneshot(
        "POST",
        &format!("/api/me/featured/{photo_id}"),
        Some(cookie),
        None,
    )
    .await;
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reorder_moves_photos_to_supplied_positions() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let mut ids = vec![];
    for i in 0..3 {
        let short = format!("AAAA{i:04}");
        ids.push(app.ready_photo_with(uid, &short, None).await);
    }
    for id in &ids {
        pin(&app, &cookie, *id).await;
    }

    let body = serde_json::json!({ "photo_ids": [ids[2], ids[1], ids[0]] });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/featured/order", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool).await.unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, ids[2]);
    assert_eq!(rows[1].id, ids[1]);
    assert_eq!(rows[2].id, ids[0]);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reorder_400_when_a_photo_is_not_currently_pinned() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let pinned = app.ready_photo_with(uid, "PINN0001", None).await;
    let unpinned = app.ready_photo_with(uid, "UNPI0001", None).await;
    pin(&app, &cookie, pinned).await;

    let body = serde_json::json!({ "photo_ids": [pinned, unpinned] });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/featured/order", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reorder_400_when_duplicate_id() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    pin(&app, &cookie, p1).await;
    pin(&app, &cookie, p2).await;

    let body = serde_json::json!({ "photo_ids": [p1, p1] });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/featured/order", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn reorder_400_when_more_than_six() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let body = serde_json::json!({
        "photo_ids": (0..7).map(|_| Uuid::new_v4()).collect::<Vec<_>>()
    });
    let (status, _) = app
        .oneshot("PATCH", "/api/me/featured/order", Some(&cookie), Some(body))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
