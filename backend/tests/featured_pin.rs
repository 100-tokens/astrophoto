mod common;

use axum::http::StatusCode;
use common::TestApp;

async fn pin(app: &TestApp, cookie: &str, photo_id: uuid::Uuid) -> StatusCode {
    let (status, _) = app
        .oneshot(
            "POST",
            &format!("/api/me/featured/{photo_id}"),
            Some(cookie),
            None,
        )
        .await;
    status
}

async fn unpin(app: &TestApp, cookie: &str, photo_id: uuid::Uuid) -> StatusCode {
    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/me/featured/{photo_id}"),
            Some(cookie),
            None,
        )
        .await;
    status
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pin_assigns_position_one_then_two() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;

    assert_eq!(pin(&app, &cookie, p1).await, StatusCode::NO_CONTENT);
    assert_eq!(pin(&app, &cookie, p2).await, StatusCode::NO_CONTENT);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].featured_position, Some(1));
    assert_eq!(rows[1].featured_position, Some(2));
    assert_eq!(rows[0].id, p1);
    assert_eq!(rows[1].id, p2);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pin_idempotent_for_already_pinned_photo() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p = app.ready_photo_with(uid, "AAAA0001", None).await;

    assert_eq!(pin(&app, &cookie, p).await, StatusCode::NO_CONTENT);
    assert_eq!(pin(&app, &cookie, p).await, StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar!(
        "select count(*) from photos where owner_id=$1 and featured_at is not null",
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 1);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pin_409_when_six_already_pinned() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let mut ids = vec![];
    for i in 0..6 {
        let short = format!("PHOT{i:04}");
        ids.push(app.ready_photo_with(uid, &short, None).await);
    }
    for id in &ids {
        assert_eq!(pin(&app, &cookie, *id).await, StatusCode::NO_CONTENT);
    }
    let extra = app.ready_photo_with(uid, "EXTR0001", None).await;
    assert_eq!(pin(&app, &cookie, extra).await, StatusCode::CONFLICT);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pin_404_when_not_owner() {
    let app = TestApp::launch().await;
    let (a_cookie, _) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let (_b_cookie, b_id) = app.signup_with_handle("B", "bob", "b@x.test").await;
    let p = app.ready_photo_with(b_id, "BOBP0001", None).await;
    assert_eq!(pin(&app, &a_cookie, p).await, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn unpin_compacts_positions() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    let p3 = app.ready_photo_with(uid, "CCCC0003", None).await;
    pin(&app, &cookie, p1).await;
    pin(&app, &cookie, p2).await;
    pin(&app, &cookie, p3).await;

    assert_eq!(unpin(&app, &cookie, p2).await, StatusCode::NO_CONTENT);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, p1);
    assert_eq!(rows[0].featured_position, Some(1));
    assert_eq!(rows[1].id, p3);
    assert_eq!(rows[1].featured_position, Some(2));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn unpin_idempotent_for_unpinned_photo() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p = app.ready_photo_with(uid, "AAAA0001", None).await;
    assert_eq!(unpin(&app, &cookie, p).await, StatusCode::NO_CONTENT);
}
