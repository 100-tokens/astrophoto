//! Deletion grace period = immediate public delisting.
//!
//! Requesting account deletion sets users.pending_deletion_at (7-day
//! grace). From that moment the account and its content must vanish
//! from every PUBLIC surface — while the owner keeps authenticated
//! access to their own data (they can still cancel). Cancelling clears
//! one column and everything reappears: no per-photo flags, no rebuild.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

struct World {
    app: TestApp,
    cookie: String,
    uid: Uuid,
    photo: Uuid,
    short_id: String,
}

#[allow(clippy::unwrap_used)]
async fn world() -> World {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Ghost", "ghost", "ghost@x.test")
        .await;
    let photo = app.ready_photo_with(uid, "GHST0001", Some("M31")).await;
    sqlx::query!(
        "update photos set processing_json = '{\"pipeline\":[]}'::jsonb where id = $1",
        photo
    )
    .execute(&app.pool)
    .await
    .unwrap();
    // A follower, so the photographers card exists pre-deletion.
    let (_, fan) = app.signup_with_handle("Fan", "fan", "fan@x.test").await;
    sqlx::query!(
        "insert into follows (follower_id, followed_id) values ($1, $2)",
        fan,
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();
    World {
        app,
        cookie,
        uid,
        photo,
        short_id: "GHST0001".into(),
    }
}

#[allow(clippy::unwrap_used)]
async fn set_pending(app: &TestApp, uid: Uuid, pending: bool) {
    if pending {
        sqlx::query!(
            "update users set pending_deletion_at = now() + interval '7 days' where id = $1",
            uid
        )
        .execute(&app.pool)
        .await
        .unwrap();
    } else {
        sqlx::query!(
            "update users set pending_deletion_at = null where id = $1",
            uid
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }
}

async fn status_of(app: &TestApp, uri: &str, cookie: Option<&str>) -> StatusCode {
    let (status, _) = app.oneshot("GET", uri, cookie, None).await;
    status
}

async fn body_of(app: &TestApp, uri: &str) -> String {
    let (_, v) = app
        .oneshot_json::<serde_json::Value>("GET", uri, None, None)
        .await;
    v.to_string()
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn grace_period_delists_every_public_surface_and_cancel_restores() {
    let w = world().await;
    let app = &w.app;

    // ── Before: everything public. (Markers avoid incidental matches
    // — /api/search echoes the query string back verbatim.) ─────────
    assert!(body_of(app, "/api/explore").await.contains("GHST0001"));
    assert!(
        body_of(app, "/api/photographers")
            .await
            .contains("\"handle\":\"ghost\"")
    );
    assert!(
        body_of(app, "/api/search?q=ghost")
            .await
            .contains("\"handle\":\"ghost\"")
    );
    assert!(
        body_of(app, "/api/photos?limit=24")
            .await
            .contains("GHST0001")
    );
    for uri in [
        "/api/users/by-handle/ghost/profile",
        "/api/users/by-handle/ghost",
        "/api/users/by-handle/ghost/photos",
        "/api/photos/by-permalink/ghost/GHST0001",
        &format!("/api/users/{}", w.uid),
        &format!("/api/photos/{}", w.photo),
        &format!("/api/users/{}/followers/count", w.uid),
    ] {
        assert_eq!(
            status_of(app, uri, None).await,
            StatusCode::OK,
            "pre-deletion: {uri}"
        );
    }

    // ── Request deletion: instant public delisting. ────────────────
    set_pending(app, w.uid, true).await;

    for (feed, marker) in [
        ("/api/explore", "GHST0001"),
        ("/api/photographers", "\"handle\":\"ghost\""),
        ("/api/search?q=ghost", "\"handle\":\"ghost\""),
        ("/api/photos?limit=24", "GHST0001"),
    ] {
        assert!(
            !body_of(app, feed).await.contains(marker),
            "grace: {feed} must not list the account"
        );
    }
    for uri in [
        "/api/users/by-handle/ghost/profile",
        "/api/users/by-handle/ghost",
        "/api/users/by-handle/ghost/photos",
        "/api/photos/by-permalink/ghost/GHST0001",
        &format!("/api/users/{}", w.uid),
        &format!("/api/photos/{}", w.photo),
        &format!("/api/users/{}/followers/count", w.uid),
    ] {
        assert_eq!(
            status_of(app, uri, None).await,
            StatusCode::NOT_FOUND,
            "grace: {uri} must 404 for the public"
        );
    }
    // Processing history hides like the photo (null body, not data).
    let (_, processing) = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/photos/{}/processing", w.photo),
            None,
            None,
        )
        .await;
    assert!(processing.is_null(), "processing hidden for anon");

    // ── The owner keeps access to their own data. ──────────────────
    assert_eq!(
        status_of(app, &format!("/api/photos/{}", w.photo), Some(&w.cookie)).await,
        StatusCode::OK,
        "owner still reads their own photo"
    );
    let own_frames = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/photos?owner_id={}&limit=24", w.uid),
            Some(&w.cookie),
            None,
        )
        .await
        .1
        .to_string();
    assert!(
        own_frames.contains("GHST0001"),
        "/account/frames listing keeps working for the owner: {own_frames}"
    );
    // But the same query is empty for the public.
    assert!(
        !body_of(app, &format!("/api/photos?owner_id={}&limit=24", w.uid))
            .await
            .contains("GHST0001"),
        "public owner_id listing hidden"
    );

    // ── Cancel: one column write restores everything. ──────────────
    set_pending(app, w.uid, false).await;
    assert!(body_of(app, "/api/explore").await.contains("GHST0001"));
    assert!(
        body_of(app, "/api/photographers")
            .await
            .contains("\"handle\":\"ghost\"")
    );
    assert_eq!(
        status_of(app, "/api/users/by-handle/ghost/profile", None).await,
        StatusCode::OK
    );
    assert_eq!(
        status_of(app, "/api/photos/by-permalink/ghost/GHST0001", None).await,
        StatusCode::OK
    );
    let (_, followers) = app
        .oneshot_json::<serde_json::Value>(
            "GET",
            &format!("/api/users/{}/followers/count", w.uid),
            None,
            None,
        )
        .await;
    assert_eq!(
        followers["count"], 1,
        "follower rows survived the grace period"
    );
    let _ = w.short_id;
}
