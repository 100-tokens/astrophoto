//! OAuth callback state validation [audit TEST-2] — boundary only.
//!
//! `GET /api/auth/oauth/google/callback` must reject any request that
//! fails the CSRF state check BEFORE attempting the Google code
//! exchange. The exchange itself (and the `email_verified` gate inside
//! `upsert_oauth_user`) needs a live Google token endpoint, so those
//! paths are exercised only up to that boundary: with the state check
//! satisfied, the handler proceeds and fails on the (unconfigured)
//! Google client — a 500, NOT a 422 — proving the gate was passed.
//!
//! Cookie name: tests run with `session_secure=false`, so the state
//! cookie is the unprefixed `oauth-state` (see `auth/oauth.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use common::TestApp;
use serde_json::Value as Json;

const CALLBACK: &str = "/api/auth/oauth/google/callback";

/// Build an `oauth-state=<b64>` cookie carrying the given CSRF token,
/// in the exact shape `oauth_google::start` writes.
fn state_cookie(csrf_token: &str) -> String {
    let payload = serde_json::json!({
        "csrf_token": csrf_token,
        "pkce_verifier": "test-pkce-verifier",
    });
    format!(
        "oauth-state={}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap())
    )
}

fn message_of(body: &[u8]) -> String {
    let v: Json = serde_json::from_slice(body).unwrap();
    v["message"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn callback_error_param_short_circuits_with_422() {
    let app = TestApp::launch().await;
    // Even with a plausible code+state, error= must win.
    let (status, body) = app
        .oneshot(
            "GET",
            &format!("{CALLBACK}?error=access_denied&code=x&state=y"),
            None,
            None,
        )
        .await;
    assert_eq!(status, 422);
    assert!(
        message_of(&body).contains("oauth error"),
        "body: {}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn callback_missing_code_or_state_returns_422() {
    let app = TestApp::launch().await;

    let (status, body) = app
        .oneshot("GET", &format!("{CALLBACK}?state=y"), None, None)
        .await;
    assert_eq!(status, 422);
    assert!(message_of(&body).contains("missing code"));

    let (status, body) = app
        .oneshot("GET", &format!("{CALLBACK}?code=x"), None, None)
        .await;
    assert_eq!(status, 422);
    assert!(message_of(&body).contains("missing state"));
}

#[tokio::test]
async fn callback_missing_state_cookie_returns_422() {
    let app = TestApp::launch().await;
    let (status, body) = app
        .oneshot("GET", &format!("{CALLBACK}?code=x&state=y"), None, None)
        .await;
    assert_eq!(status, 422);
    assert!(
        message_of(&body).contains("missing state cookie"),
        "body: {}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn callback_malformed_state_cookie_returns_422() {
    let app = TestApp::launch().await;

    // Not base64 at all.
    let (status, body) = app
        .oneshot(
            "GET",
            &format!("{CALLBACK}?code=x&state=y"),
            Some("oauth-state=!!!not-base64!!!"),
            None,
        )
        .await;
    assert_eq!(status, 422);
    assert!(message_of(&body).contains("bad state cookie"));

    // Valid base64, but the payload is not an OauthState JSON document.
    let garbage = format!("oauth-state={}", URL_SAFE_NO_PAD.encode(b"not json"));
    let (status, body) = app
        .oneshot(
            "GET",
            &format!("{CALLBACK}?code=x&state=y"),
            Some(&garbage),
            None,
        )
        .await;
    assert_eq!(status, 422);
    assert!(message_of(&body).contains("bad state payload"));
}

#[tokio::test]
async fn callback_state_mismatch_returns_422() {
    let app = TestApp::launch().await;
    let cookie = state_cookie("expected-token");
    let (status, body) = app
        .oneshot(
            "GET",
            &format!("{CALLBACK}?code=x&state=attacker-token"),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(status, 422);
    assert!(
        message_of(&body).contains("state mismatch"),
        "body: {}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn callback_matching_state_passes_gate_and_stops_at_google_boundary() {
    let app = TestApp::launch().await;
    let cookie = state_cookie("matching-token");
    let (status, body) = app
        .oneshot(
            "GET",
            &format!("{CALLBACK}?code=x&state=matching-token"),
            Some(&cookie),
            None,
        )
        .await;
    // With state validated, the handler builds the Google client — the
    // test config has no client id, so it fails as 500 Internal. The
    // important assertion: NOT 422, i.e. the CSRF gate was cleared.
    assert_eq!(
        status,
        500,
        "matching state must get past validation; body: {}",
        String::from_utf8_lossy(&body)
    );
    let v: Json = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "internal");
}
