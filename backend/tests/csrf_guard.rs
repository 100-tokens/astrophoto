//! Integration tests for the Origin/Referer CSRF guard (http::csrf::origin_guard).
//! The guard is a global layer added in main.rs, so the normal app harness
//! (which builds only http::router) doesn't exercise it — this wraps a minimal
//! router with the layer directly. No DB needed: the guard inspects headers only.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use astrophoto::http::csrf::{AllowedOrigins, origin_guard};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    middleware::from_fn_with_state,
    routing::post,
};
use tower::ServiceExt;

fn app() -> Router {
    let allowed = AllowedOrigins(HashSet::from(["https://app.example".to_string()]));
    Router::new()
        .route(
            "/m",
            post(|| async { StatusCode::OK }).get(|| async { StatusCode::OK }),
        )
        .layer(from_fn_with_state(allowed, origin_guard))
}

const SESSION: &str = "__Host-session=deadbeef";

async fn status(req: Request<Body>) -> StatusCode {
    app().oneshot(req).await.unwrap().status()
}

fn req(method: &str, headers: &[(header::HeaderName, &str)]) -> Request<Body> {
    let mut b = Request::builder().method(method).uri("/m");
    for (k, v) in headers {
        b = b.header(k.clone(), *v);
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn allows_cookie_post_from_allowed_origin() {
    let s = status(req(
        "POST",
        &[(header::COOKIE, SESSION), (header::ORIGIN, "https://app.example")],
    ))
    .await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn blocks_cookie_post_from_foreign_origin() {
    let s = status(req(
        "POST",
        &[(header::COOKIE, SESSION), (header::ORIGIN, "https://evil.example")],
    ))
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allows_cookie_post_with_no_origin_or_referer() {
    // Trusted server-side caller (SSR event.fetch) — no Origin, no Referer.
    let s = status(req("POST", &[(header::COOKIE, SESSION)])).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn referer_fallback_blocks_foreign_and_allows_self() {
    let foreign = status(req(
        "POST",
        &[(header::COOKIE, SESSION), (header::REFERER, "https://evil.example/page")],
    ))
    .await;
    assert_eq!(foreign, StatusCode::FORBIDDEN);

    let own = status(req(
        "POST",
        &[(header::COOKIE, SESSION), (header::REFERER, "https://app.example/page")],
    ))
    .await;
    assert_eq!(own, StatusCode::OK);
}

#[tokio::test]
async fn allows_cookieless_post_from_foreign_origin() {
    // Anonymous endpoints (login/signup/reset) carry no session cookie and
    // can't be CSRF'd into acting as a victim — the guard must not touch them.
    let s = status(req("POST", &[(header::ORIGIN, "https://evil.example")])).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn allows_get_regardless_of_origin() {
    // GET (e.g. the OAuth callback class) is never a CSRF-relevant mutation.
    let s = status(req(
        "GET",
        &[(header::COOKIE, SESSION), (header::ORIGIN, "https://evil.example")],
    ))
    .await;
    assert_eq!(s, StatusCode::OK);
}
