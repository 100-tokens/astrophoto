//! GET /api/photographers — first test coverage for the endpoint
//! (every sibling discovery route already had a file). Pins the
//! 33e2575 stats-CTE fanout fix, the strict cursor/sort contract, and
//! keyset pagination over the tie-heavy data real sites have.

mod common;

use astrophoto::api_types::PhotographerIndexPage;
use axum::http::StatusCode;
use common::TestApp;
use uuid::Uuid;

async fn follow(app: &TestApp, follower: Uuid, followed: Uuid) {
    #[allow(clippy::unwrap_used)]
    sqlx::query!(
        "insert into follows (follower_id, followed_id) values ($1, $2)",
        follower,
        followed
    )
    .execute(&app.pool)
    .await
    .unwrap();
}

/// The photos × follows fanout regression (33e2575): a followed
/// photographer's frame count and integration must not multiply by
/// follower count, on any sort. Guarded only by a SQL comment until now.
#[tokio::test]
#[allow(clippy::unwrap_used, clippy::panic)]
async fn stats_do_not_fan_out_with_followers() {
    let app = TestApp::launch().await;
    let (_, star) = app.signup_with_handle("Star", "star", "star@x.test").await;
    let p1 = app.ready_photo_with(star, "STAR0001", None).await;
    let p2 = app.ready_photo_with(star, "STAR0002", None).await;
    sqlx::query!("update photos set integration_s = 401400 where id = $1", p1)
        .execute(&app.pool)
        .await
        .unwrap();
    sqlx::query!(
        "update photos set exposure_s = 300, sessions = 10 where id = $1",
        p2
    )
    .execute(&app.pool)
    .await
    .unwrap();
    // Three followers — the old join fanned photos × follows.
    for (i, h) in ["fana", "fanb", "fanc"].iter().enumerate() {
        let (_, f) = app
            .signup_with_handle("Fan", h, &format!("fan{i}@x.test"))
            .await;
        follow(&app, f, star).await;
    }

    for sort in ["active", "followers", "recent"] {
        let (status, body) = app
            .oneshot_json::<PhotographerIndexPage>(
                "GET",
                &format!("/api/photographers?sort={sort}"),
                None,
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        let item = body
            .items
            .iter()
            .find(|i| i.handle == "star")
            .unwrap_or_else(|| panic!("star listed under {sort}"));
        assert_eq!(item.frame_count, 2, "frames not multiplied ({sort})");
        assert_eq!(item.follower_count, 3, "follower count ({sort})");
        assert_eq!(
            item.integration_seconds, 404_400,
            "integration not multiplied ({sort})"
        );
    }
}

/// Unknown sorts, garbage cursors, and cross-sort cursor replay are
/// 400s — they used to be silently served (wrong feed / page 1 / a
/// wrong keyset slice, since active and followers share a cursor shape).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn strict_sort_and_cursor_contract() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("A", "alice", "a@x.test").await;
    // Three photographers so limit=2 emits a cursor.
    for (i, h) in ["bob", "carol"].iter().enumerate() {
        let (_, other) = app
            .signup_with_handle("U", h, &format!("u{i}@x.test"))
            .await;
        app.ready_photo_with(other, &format!("PHZZ000{i}"), None)
            .await;
    }
    app.ready_photo_with(uid, "PHZZ0009", None).await;

    let (status, _) = app
        .oneshot("GET", "/api/photographers?sort=banana", None, None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "unknown sort");

    let (status, _) = app
        .oneshot("GET", "/api/photographers?cursor=garbage", None, None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "garbage cursor");

    // A cursor minted under active must not paginate followers (same
    // struct shape — only the embedded sort tag tells them apart).
    let (_, page1) = app
        .oneshot_json::<PhotographerIndexPage>(
            "GET",
            "/api/photographers?sort=active&limit=2",
            None,
            None,
        )
        .await;
    let c = page1.next_cursor.expect("cursor after page 1");
    let (status, _) = app
        .oneshot(
            "GET",
            &format!("/api/photographers?sort=followers&cursor={c}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "cross-sort cursor");
    let (status, _) = app
        .oneshot(
            "GET",
            &format!("/api/photographers?sort=recent&cursor={c}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "count cursor under recent");
}

/// Keyset round-trip on maximum-tie data (everyone has exactly 1 frame
/// and 0 followers — the realistic young-site distribution): every
/// photographer appears exactly once across pages, terminal page emits
/// no cursor, and a total of exactly `limit` rows emits none either.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn cursor_round_trip_with_ties_all_sorts() {
    let app = TestApp::launch().await;
    let mut handles = Vec::new();
    for i in 0..5 {
        let (_, uid) = app
            .signup_with_handle("U", &format!("tie{i}"), &format!("tie{i}@x.test"))
            .await;
        app.ready_photo_with(uid, &format!("TIEP000{i}"), None)
            .await;
        handles.push(format!("tie{i}"));
    }

    for sort in ["active", "followers", "recent"] {
        let mut seen: Vec<String> = Vec::new();
        let mut cursor: Option<String> = None;
        for _page in 0..5 {
            let url = match &cursor {
                Some(c) => format!("/api/photographers?sort={sort}&limit=2&cursor={c}"),
                None => format!("/api/photographers?sort={sort}&limit=2"),
            };
            let (status, body) = app
                .oneshot_json::<PhotographerIndexPage>("GET", &url, None, None)
                .await;
            assert_eq!(status, StatusCode::OK, "{sort}");
            seen.extend(body.items.iter().map(|i| i.handle.clone()));
            match body.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        let unique: std::collections::HashSet<&String> = seen.iter().collect();
        assert_eq!(seen.len(), 5, "no skips under {sort}: {seen:?}");
        assert_eq!(unique.len(), 5, "no duplicates under {sort}: {seen:?}");
    }

    // Exactly `limit` total rows → no spurious cursor (the old
    // rows.len()==limit heuristic emitted one, making a dead Load more).
    let (_, body) = app
        .oneshot_json::<PhotographerIndexPage>(
            "GET",
            "/api/photographers?sort=active&limit=5",
            None,
            None,
        )
        .await;
    assert_eq!(body.items.len(), 5);
    assert!(
        body.next_cursor.is_none(),
        "exact-limit page must not emit a cursor"
    );
}

/// frame_count counts only published AND ready photos — a photo
/// mid-replace (status back to 'processing') or failed must not keep a
/// photographer listed here while profile/explore disagree.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn only_ready_published_photos_count() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Ghost", "ghost", "g@x.test").await;
    let p = app.ready_photo_with(uid, "GHST0001", None).await;
    sqlx::query!("update photos set status = 'processing' where id = $1", p)
        .execute(&app.pool)
        .await
        .unwrap();

    let (_, body) = app
        .oneshot_json::<PhotographerIndexPage>("GET", "/api/photographers", None, None)
        .await;
    assert!(
        body.items.iter().all(|i| i.handle != "ghost"),
        "published-but-processing photo must not list the photographer"
    );

    // Limit clamp sanity while we're here: limit=0 → 1 row max.
    let (status, body) = app
        .oneshot_json::<PhotographerIndexPage>("GET", "/api/photographers?limit=0", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.items.len() <= 1, "limit=0 clamps to 1");
}
