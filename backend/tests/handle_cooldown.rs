//! Handle-rename cooldown [audit DB-3 regression].
//!
//! Renaming writes a `handle_redirects` row with `released_at = now() +
//! 90 days` — the instant the old handle becomes reservable again. Until
//! then another user must not be able to claim it (anti-impersonation),
//! and `/api/auth/handle-check` must report it "taken". The original
//! owner may always reclaim their own released handle. See
//! `backend/src/users/handle.rs` and `backend/src/auth/handle_check.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::TestApp;
use serde_json::{Value as Json, json};

async fn handle_status(app: &TestApp, handle: &str) -> String {
    let (status, body): (_, Json) = app
        .oneshot_json(
            "GET",
            &format!("/api/auth/handle-check?handle={handle}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, 200);
    body["status"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn released_handle_is_blocked_for_others_during_cooldown() {
    let app = TestApp::launch().await;
    let (alice_cookie, _alice) = app
        .signup_with_handle("Alice", "alice-orig", "alice@example.com")
        .await;
    let (bob_cookie, bob) = app
        .signup_with_handle("Bob", "bob-handle", "bob@example.com")
        .await;

    // Alice renames away from alice-orig, starting the 90-day cooldown.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&alice_cookie),
            Some(json!({ "handle": "alice-new" })),
        )
        .await;
    assert_eq!(status, 200, "rename must succeed");

    // The released handle reads as taken, not available.
    assert_eq!(handle_status(&app, "alice-orig").await, "taken");

    // Bob cannot claim it during the cooldown.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&bob_cookie),
            Some(json!({ "handle": "alice-orig" })),
        )
        .await;
    assert_eq!(status, 409, "cooldown handle must 409 for another user");

    // Bob's handle is unchanged.
    let bob_handle: String = sqlx::query_scalar!(
        r#"select handle::text as "h!" from users where id = $1"#,
        bob
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(bob_handle, "bob-handle");
}

#[tokio::test]
async fn expired_cooldown_frees_the_handle() {
    let app = TestApp::launch().await;
    let (alice_cookie, _alice) = app
        .signup_with_handle("Alice", "alice-orig", "alice2@example.com")
        .await;
    let (bob_cookie, bob) = app
        .signup_with_handle("Bob", "bob-handle", "bob2@example.com")
        .await;

    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&alice_cookie),
            Some(json!({ "handle": "alice-new" })),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(handle_status(&app, "alice-orig").await, "taken");

    // Backdate the cooldown: released_at in the past = reservable now.
    sqlx::query!(
        "update handle_redirects set released_at = now() - interval '1 day'
          where old_handle = 'alice-orig'"
    )
    .execute(&app.pool)
    .await
    .unwrap();

    assert_eq!(handle_status(&app, "alice-orig").await, "available");

    // Bob claims the freed handle.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&bob_cookie),
            Some(json!({ "handle": "alice-orig" })),
        )
        .await;
    assert_eq!(status, 200, "expired cooldown must allow the claim");

    let bob_handle: String = sqlx::query_scalar!(
        r#"select handle::text as "h!" from users where id = $1"#,
        bob
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(bob_handle, "alice-orig");

    // Claiming clears the stale redirect so /u/alice-orig stops pointing
    // at Alice; Bob's own released handle now has a redirect row instead.
    let stale: bool = sqlx::query_scalar!(
        r#"select exists(select 1 from handle_redirects where old_handle = 'alice-orig') as "e!""#
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(!stale, "claimed handle's redirect row must be deleted");

    let bob_redirect: bool = sqlx::query_scalar!(
        r#"select exists(select 1 from handle_redirects
                          where old_handle = 'bob-handle' and user_id = $1) as "e!""#,
        bob
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(bob_redirect, "bob's released handle must enter cooldown");
}

#[tokio::test]
async fn owner_can_reclaim_own_handle_during_cooldown() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Alice", "alice-orig", "alice3@example.com")
        .await;

    // a -> b, then back b -> a while a is still in cooldown.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&cookie),
            Some(json!({ "handle": "alice-new" })),
        )
        .await;
    assert_eq!(status, 200);

    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&cookie),
            Some(json!({ "handle": "alice-orig" })),
        )
        .await;
    assert_eq!(status, 200, "owner reclaim during cooldown must succeed");

    let handle: String = sqlx::query_scalar!(
        r#"select handle::text as "h!" from users where id = $1"#,
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(handle, "alice-orig");

    // Reclaiming clears the a-redirect; the abandoned b-handle is now the
    // one in cooldown.
    let a_redirect: bool = sqlx::query_scalar!(
        r#"select exists(select 1 from handle_redirects where old_handle = 'alice-orig') as "e!""#
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(!a_redirect, "reclaimed handle's redirect row must be gone");
    assert_eq!(handle_status(&app, "alice-new").await, "taken");
}

#[tokio::test]
async fn signup_cannot_claim_a_cooldown_handle() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app
        .signup_with_handle("Alice", "alice-orig", "alice@example.com")
        .await;

    // alice renames, releasing alice-orig into the 90-day cooldown.
    let (status, _) = app
        .oneshot(
            "POST",
            "/api/me/handle",
            Some(&cookie),
            Some(json!({ "handle": "alice-new" })),
        )
        .await;
    assert_eq!(status, 200);

    // A brand-new signup must not be able to take the reserved handle —
    // otherwise the rename-path cooldown is bypassed with a fresh account.
    let body = json!({
        "email": "mallory@example.com",
        "password": "verylongpassword",
        "display_name": "Mallory",
        "handle": "alice-orig"
    });
    let (status, resp) = app
        .oneshot("POST", "/api/auth/signup", None, Some(body.clone()))
        .await;
    assert_eq!(
        status,
        409,
        "cooldown handle must be reserved: {}",
        String::from_utf8_lossy(&resp)
    );

    // Expired cooldown frees it for signup too.
    sqlx::query!(
        "update handle_redirects set released_at = now() - interval '1 day'
          where old_handle = 'alice-orig'"
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (status, resp) = app
        .oneshot("POST", "/api/auth/signup", None, Some(body))
        .await;
    assert_eq!(
        status,
        202,
        "expired cooldown must free the handle: {}",
        String::from_utf8_lossy(&resp)
    );
}
