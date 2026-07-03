mod common;

use astrophoto::api_types::DiscoveryPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_returns_published_photos_newest_first_across_authors() {
    let app = TestApp::launch().await;
    let (_, alice_id) = app
        .signup_with_handle("Alice", "alice", "alice@x.test")
        .await;
    let (_, bob_id) = app.signup_with_handle("Bob", "bob", "bob@x.test").await;
    let _p1 = app
        .ready_photo_with(alice_id, "AAAA0001", Some("M31"))
        .await;
    let _p2 = app.ready_photo_with(bob_id, "BBBB0001", Some("M42")).await;
    let p3 = app
        .ready_photo_with(alice_id, "AAAA0002", Some("NGC 7000"))
        .await;

    let (status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=2", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.photos.len(), 2);
    assert_eq!(body.photos[0].id, p3, "newest first across owners");
    assert!(body.next_cursor.is_some(), "more pages remain");
    // Author chip data must come back.
    assert_eq!(body.photos[0].author_handle, "alice");
    assert_eq!(body.photos[0].author_display_name, "Alice");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_respects_limit_clamp() {
    let app = TestApp::launch().await;
    let (status, _) = app
        .oneshot("GET", "/api/explore?limit=999", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.oneshot("GET", "/api/explore?limit=0", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_filters_by_category() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let p_dso = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;
    let _p_lunar = app.ready_photo_with(uid, "BBBB0001", Some("Moon")).await;
    sqlx::query!("update photos set category = 'dso' where id = $1", p_dso)
        .execute(&app.pool)
        .await
        .unwrap();
    sqlx::query!(
        "update photos set category = 'lunar' where id = $1",
        _p_lunar
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?category=dso", None, None)
        .await;
    assert_eq!(body.photos.len(), 1);
    assert_eq!(body.photos[0].id, p_dso);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_sort_most_appreciated_orders_by_count() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    sqlx::query!(
        "update photos set appreciations_count = 5 where id = $1",
        p1
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?sort=most-appreciated", None, None)
        .await;
    assert_eq!(body.photos[0].id, p1);
    assert_eq!(body.photos[1].id, p2);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_following_unauthenticated_returns_empty() {
    // following=true is "people you follow"; an anonymous caller follows nobody.
    // Without this short-circuit the filter was silently ignored and the full
    // feed leaked through.
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let _ = app.ready_photo_with(uid, "AAAA0001", None).await;

    let (status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?following=true", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.photos.is_empty(),
        "anonymous following=true must return empty"
    );
    assert!(body.next_cursor.is_none());
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_following_authenticated_filters_to_followed_users() {
    let app = TestApp::launch().await;
    let (alice_cookie, alice_id) = app
        .signup_with_handle("Alice", "alice", "alice@x.test")
        .await;
    let (_, bob_id) = app.signup_with_handle("Bob", "bob", "bob@x.test").await;
    let (_, carol_id) = app
        .signup_with_handle("Carol", "carol", "carol@x.test")
        .await;
    let _alice_p = app.ready_photo_with(alice_id, "AAAA0001", None).await;
    let bob_p = app.ready_photo_with(bob_id, "BBBB0001", None).await;
    let _carol_p = app.ready_photo_with(carol_id, "CCCC0001", None).await;

    // Alice follows Bob, but not Carol or herself.
    sqlx::query!(
        "insert into follows (follower_id, followed_id) values ($1, $2)",
        alice_id,
        bob_id
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, body) = app
        .oneshot_json::<DiscoveryPage>(
            "GET",
            "/api/explore?following=true",
            Some(&alice_cookie),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.photos.len(), 1, "only Bob's photo should match");
    assert_eq!(body.photos[0].id, bob_p);
}

// ── Surface-audit additions ──────────────────────────────────────────

/// Unknown enum-ish params are consistent 400s. `sort` used to silently
/// serve the newest feed; `category` silently served an empty feed —
/// indistinguishable from an empty category (a typo'd link like
/// ?category=widefield rendered a permanently blank page).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_rejects_unknown_sort_and_category() {
    let app = TestApp::launch().await;
    for q in ["sort=bogus", "category=bogus", "since=bogus"] {
        let (status, _) = app
            .oneshot("GET", &format!("/api/explore?{q}"), None, None)
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q} must 400");
    }
    // The full valid sets still work.
    for q in [
        "sort=newest",
        "sort=most-appreciated",
        "since=24h",
        "since=7d",
        "since=30d",
        "since=all",
        "category=dso",
        "category=other",
    ] {
        let (status, _) = app
            .oneshot("GET", &format!("/api/explore?{q}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK, "{q} must be accepted");
    }
}

/// The limit clamp actually applies (the old test only asserted 200 —
/// it would pass with the clamp deleted).
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_limit_clamp_applies() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    for i in 0..3 {
        app.ready_photo_with(uid, &format!("CLMP000{i}"), None)
            .await;
    }
    let (_, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=0", None, None)
        .await;
    assert_eq!(body.photos.len(), 1, "limit=0 clamps to 1");
    let (_, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=999", None, None)
        .await;
    assert_eq!(
        body.photos.len(),
        3,
        "limit=999 clamps to 60, returns all 3"
    );
}

/// Cursor round-trip under sort=newest: page 2 continues exactly where
/// page 1 stopped — no duplicates, no skips, terminal page has no cursor.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_cursor_round_trip_newest() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let mut ids = Vec::new();
    for i in 0..5 {
        let id = app
            .ready_photo_with(uid, &format!("CRSR000{i}"), None)
            .await;
        // Distinct published_at so ordering is deterministic.
        sqlx::query!(
            "update photos set published_at = now() - ($2 || ' minutes')::interval where id = $1",
            id,
            (i + 1).to_string()
        )
        .execute(&app.pool)
        .await
        .unwrap();
        ids.push(id);
    }

    let (_, page1) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=2", None, None)
        .await;
    assert_eq!(page1.photos.len(), 2);
    let c1 = page1.next_cursor.expect("cursor after page 1");
    let (_, page2) = app
        .oneshot_json::<DiscoveryPage>(
            "GET",
            &format!("/api/explore?limit=2&cursor={c1}"),
            None,
            None,
        )
        .await;
    assert_eq!(page2.photos.len(), 2);
    let c2 = page2.next_cursor.expect("cursor after page 2");
    let (_, page3) = app
        .oneshot_json::<DiscoveryPage>(
            "GET",
            &format!("/api/explore?limit=2&cursor={c2}"),
            None,
            None,
        )
        .await;
    assert_eq!(page3.photos.len(), 1, "final partial page");
    assert!(page3.next_cursor.is_none(), "terminal page has no cursor");

    let walked: Vec<uuid::Uuid> = [&page1.photos[..], &page2.photos[..], &page3.photos[..]]
        .concat()
        .iter()
        .map(|p| p.id)
        .collect();
    let unique: std::collections::HashSet<&uuid::Uuid> = walked.iter().collect();
    assert_eq!(unique.len(), 5, "no duplicates across pages: {walked:?}");
}

/// Cursor round-trip under sort=most-appreciated, and the cross-sort
/// mismatch: a newest cursor replayed under most-appreciated used to
/// null the keyset predicate and silently re-serve page 1.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_cursor_most_appreciated_and_sort_mismatch() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    for i in 0..4 {
        let id = app
            .ready_photo_with(uid, &format!("APPR000{i}"), None)
            .await;
        sqlx::query!(
            "update photos set appreciations_count = $2 where id = $1",
            id,
            i32::from(10 - i)
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }

    let (_, page1) = app
        .oneshot_json::<DiscoveryPage>(
            "GET",
            "/api/explore?sort=most-appreciated&limit=2",
            None,
            None,
        )
        .await;
    assert_eq!(page1.photos.len(), 2);
    assert_eq!(page1.photos[0].appreciations_count, 10);
    let c1 = page1.next_cursor.expect("cursor after page 1");
    let (_, page2) = app
        .oneshot_json::<DiscoveryPage>(
            "GET",
            &format!("/api/explore?sort=most-appreciated&limit=2&cursor={c1}"),
            None,
            None,
        )
        .await;
    assert_eq!(page2.photos.len(), 2);
    let page1_ids: std::collections::HashSet<_> = page1.photos.iter().map(|p| &p.id).collect();
    assert!(
        page2.photos.iter().all(|p| !page1_ids.contains(&p.id)),
        "page 2 must not repeat page 1"
    );

    // Garbage cursor → 400.
    let (status, _) = app
        .oneshot("GET", "/api/explore?cursor=garbage", None, None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A newest cursor (no appreciations component) under
    // most-appreciated → 400 instead of silent page-1 duplicates.
    let (_, newest_page) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=2", None, None)
        .await;
    let newest_cursor = newest_page.next_cursor.expect("newest cursor");
    let (status, _) = app
        .oneshot(
            "GET",
            &format!("/api/explore?sort=most-appreciated&cursor={newest_cursor}"),
            None,
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "cross-sort cursor rejected"
    );
}

/// The since window actually filters (the interval SQL had zero tests):
/// a 9-day-old photo is hidden by 7d/24h, shown by 30d/all.
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_since_window_filters_backdated_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let old = app.ready_photo_with(uid, "OLDP0001", None).await;
    sqlx::query!(
        "update photos set published_at = now() - interval '9 days' where id = $1",
        old
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let fresh = app.ready_photo_with(uid, "NEWP0001", None).await;

    for (q, expected) in [
        ("since=24h", vec![&fresh]),
        ("since=7d", vec![&fresh]),
        ("since=30d", vec![&fresh, &old]),
        ("since=all", vec![&fresh, &old]),
    ] {
        let (_, body) = app
            .oneshot_json::<DiscoveryPage>("GET", &format!("/api/explore?{q}"), None, None)
            .await;
        let got: Vec<&uuid::Uuid> = body.photos.iter().map(|p| &p.id).collect();
        assert_eq!(got, expected, "window {q}");
    }
}
