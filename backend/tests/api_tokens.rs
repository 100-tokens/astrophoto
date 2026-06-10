//! Personal access token (PAT) auth guards [audit TEST-1].
//!
//! Covers `backend/src/auth/middleware.rs` (Bearer branch of `resolve`,
//! `SessionOnly`, `AdminUser` PAT rejection) and the `/api/me/tokens`
//! management handlers in `backend/src/auth/tokens.rs`.
//!
//! NOTE: `api_tokens` has no expiry column (migration 0033) — tokens live
//! until revoked — so there is no "expired PAT" case to test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::TestApp;
use serde_json::{Value as Json, json};

/// Mint a PAT through the HTTP surface using a session cookie.
/// Returns (token_id, secret).
async fn mint_pat(app: &TestApp, cookie: &str, name: &str) -> (String, String) {
    let (status, body): (_, Json) = app
        .oneshot_json(
            "POST",
            "/api/me/tokens",
            Some(cookie),
            Some(json!({ "name": name })),
        )
        .await;
    assert_eq!(status, 200, "minting a PAT with a session must succeed");
    let id = body["id"].as_str().unwrap().to_string();
    let secret = body["secret"].as_str().unwrap().to_string();
    assert!(
        secret.starts_with("astrophoto_pat_"),
        "secret must carry the PAT prefix, got: {secret}"
    );
    (id, secret)
}

#[tokio::test]
async fn valid_pat_authenticates_normal_api_call() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app
        .signup_with_handle("Pat User", "patuser", "pat@example.com")
        .await;
    let (_id, secret) = mint_pat(&app, &cookie, "pixinsight").await;

    // Bearer-only request (no cookie) against a CurrentUser-guarded route.
    let (status, body) = app
        .oneshot_bearer("GET", "/api/auth/me", &secret, None)
        .await;
    assert_eq!(status, 200, "valid PAT must authenticate /api/auth/me");
    let me: Json = serde_json::from_slice(&body).unwrap();
    assert_eq!(me["email"], "pat@example.com");
    assert_eq!(me["handle"], "patuser");

    // The successful bearer call must have stamped last_used_at, visible
    // through the (session-authenticated) list endpoint.
    let (status, list): (_, Json) = app
        .oneshot_json("GET", "/api/me/tokens", Some(&cookie), None)
        .await;
    assert_eq!(status, 200);
    assert!(
        list[0]["last_used_at"].is_string(),
        "last_used_at should be set after first bearer use, got: {list}"
    );
}

#[tokio::test]
async fn revoked_pat_returns_401() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app
        .signup_with_handle("Pat User", "revoker", "revoke@example.com")
        .await;
    let (id, secret) = mint_pat(&app, &cookie, "to-revoke").await;

    // Sanity: works before revocation.
    let (status, _) = app
        .oneshot_bearer("GET", "/api/auth/me", &secret, None)
        .await;
    assert_eq!(status, 200);

    // Revoke via the session-authenticated management endpoint.
    let (status, _) = app
        .oneshot(
            "DELETE",
            &format!("/api/me/tokens/{id}"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 204, "revoke must return 204");

    let (status, _) = app
        .oneshot_bearer("GET", "/api/auth/me", &secret, None)
        .await;
    assert_eq!(status, 401, "revoked PAT must no longer authenticate");
}

#[tokio::test]
async fn garbage_bearer_tokens_return_401() {
    let app = TestApp::launch().await;
    let (_cookie, _uid) = app
        .signup_with_handle("Pat User", "garbler", "garble@example.com")
        .await;

    // Wrong scheme content entirely (no PAT prefix): falls through to the
    // cookie path, no cookie present -> 401.
    let (status, _) = app
        .oneshot_bearer("GET", "/api/auth/me", "not-a-real-token", None)
        .await;
    assert_eq!(status, 401, "non-PAT bearer must not authenticate");

    // Correct prefix but unknown secret: hash lookup misses -> 401.
    let (status, _) = app
        .oneshot_bearer(
            "GET",
            "/api/auth/me",
            "astrophoto_pat_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            None,
        )
        .await;
    assert_eq!(status, 401, "unknown PAT secret must not authenticate");
}

#[tokio::test]
async fn pat_rejected_by_session_only_routes() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app
        .signup_with_handle("Pat User", "sessonly", "sessonly@example.com")
        .await;
    let (id, secret) = mint_pat(&app, &cookie, "escalation-attempt").await;

    // PAT management itself is session-only: a stolen token must not be
    // able to mint or revoke tokens.
    let (status, _) = app
        .oneshot_bearer(
            "POST",
            "/api/me/tokens",
            &secret,
            Some(json!({ "name": "minted-by-pat" })),
        )
        .await;
    assert_eq!(status, 403, "PAT must not mint new PATs");

    let (status, _) = app
        .oneshot_bearer("GET", "/api/me/tokens", &secret, None)
        .await;
    assert_eq!(status, 403, "PAT must not list PATs");

    let (status, _) = app
        .oneshot_bearer("DELETE", &format!("/api/me/tokens/{id}"), &secret, None)
        .await;
    assert_eq!(status, 403, "PAT must not revoke PATs");

    // Account-takeover surface: password change is session-only.
    let (status, _) = app
        .oneshot_bearer(
            "POST",
            "/api/me/password-change",
            &secret,
            Some(json!({
                "current_password": "verylongpassword",
                "new_password": "anotherlongpassword"
            })),
        )
        .await;
    assert_eq!(status, 403, "PAT must not change the password");

    // The token must still be alive after all those rejections.
    let (status, _) = app
        .oneshot_bearer("GET", "/api/auth/me", &secret, None)
        .await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn pat_rejected_by_admin_routes_even_for_admin_user() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Admin User", "adminpat", "adminpat@example.com")
        .await;
    let (_id, secret) = mint_pat(&app, &cookie, "admin-token").await;

    sqlx::query!("update users set is_admin = true where id = $1", uid)
        .execute(&app.pool)
        .await
        .unwrap();

    // Session works on the admin surface…
    let (status, _) = app
        .oneshot("GET", "/api/admin/equipment", Some(&cookie), None)
        .await;
    assert_eq!(status, 200, "admin session must reach admin routes");

    // …but the same user's PAT must be rejected with 403.
    let (status, _) = app
        .oneshot_bearer("GET", "/api/admin/equipment", &secret, None)
        .await;
    assert_eq!(status, 403, "PAT must never satisfy an admin route");
}
