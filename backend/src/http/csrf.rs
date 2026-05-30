//! Origin/Referer CSRF guard for cookie-authenticated mutating requests.
//!
//! Why this exists even though browser→API traffic is proxied same-origin:
//! the session cookie is `SameSite=None` in prod (frontend and backend are
//! sibling subdomains), and OAuth users additionally hold a backend-origin
//! session cookie. So a cross-site page can drive a state-changing request on
//! a logged-in victim's behalf two ways — (A) a bodyless `fetch` through the
//! proxy (a CORS-simple request SvelteKit's form-only CSRF guard ignores), and
//! (B) a multipart POST straight to the backend, bypassing the proxy entirely.
//! CORS does not stop either (it blocks reading the response, not executing the
//! request). This guard rejects mutating requests whose Origin/Referer is a
//! cross-site host, while leaving every legitimate flow untouched.
//!
//! Allow rule: a mutating + cookie-bearing request is allowed iff its Origin
//! (or, absent Origin, its Referer's origin) is in the allowlist OR is absent
//! entirely. Absent is allowed because a browser ALWAYS sends Origin on a
//! cross-origin state change — a missing Origin/Referer means a trusted
//! server-side caller (SSR `event.fetch`), never a cross-site browser request.

use std::collections::HashSet;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::auth::session;
use crate::error::AppError;

/// The set of browser-facing frontend origins permitted to drive cookie-auth
/// mutations. Seeded from `APP_CORS_ORIGIN` plus any `APP_EXTRA_BROWSER_ORIGINS`.
#[derive(Clone)]
pub struct AllowedOrigins(pub HashSet<String>);

/// Extract `scheme://authority` from an absolute URL string (for the Referer
/// fallback). Returns None for relative / scheme-only / malformed input.
pub fn origin_of(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let authority_start = scheme_end + 3;
    if authority_start >= url.len() {
        return None;
    }
    let authority_end = url[authority_start..]
        .find('/')
        .map(|i| authority_start + i)
        .unwrap_or(url.len());
    let origin = &url[..authority_end];
    // Must have a non-empty authority after the scheme.
    if authority_end <= authority_start {
        return None;
    }
    Some(origin.to_string())
}

/// True if the request carries a session cookie. Mirrors the session
/// extractor's parse (split on ';', trim, match COOKIE_NAMES exactly).
fn has_session_cookie(headers: &HeaderMap) -> bool {
    let Some(cookie_str) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    cookie_str.split(';').map(str::trim).any(|kv| {
        session::COOKIE_NAMES
            .iter()
            .any(|name| kv.starts_with(&format!("{name}=")))
    })
}

pub async fn origin_guard(
    State(allowed): State<AllowedOrigins>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // Only state-changing methods are CSRF-relevant. GET/HEAD/OPTIONS pass.
    let is_mutating = matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if !is_mutating {
        return Ok(next.run(request).await);
    }

    let headers = request.headers();
    // Anonymous endpoints (login, signup, password-reset, OAuth start) carry no
    // session cookie — they can't be CSRF'd into acting as a victim. Skip them
    // so this guard never affects unauthenticated flows.
    if !has_session_cookie(headers) {
        return Ok(next.run(request).await);
    }

    let claimed = headers
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get(header::REFERER)
                .and_then(|h| h.to_str().ok())
                .and_then(origin_of)
        });

    match claimed {
        // No Origin and no Referer → trusted server-side caller (SSR fetch);
        // a cross-site browser request would always carry Origin.
        None => Ok(next.run(request).await),
        Some(o) if allowed.0.contains(&o) => Ok(next.run(request).await),
        Some(_) => Err(AppError::Forbidden),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_of_extracts_scheme_and_authority() {
        assert_eq!(
            origin_of("https://www.example.com/path?q=1").as_deref(),
            Some("https://www.example.com")
        );
        assert_eq!(
            origin_of("https://www.example.com").as_deref(),
            Some("https://www.example.com")
        );
        assert_eq!(
            origin_of("http://localhost:5173/api/x").as_deref(),
            Some("http://localhost:5173")
        );
    }

    #[test]
    fn origin_of_rejects_non_absolute() {
        assert_eq!(origin_of("/relative/path"), None);
        assert_eq!(origin_of("notaurl"), None);
        assert_eq!(origin_of("https://"), None);
        assert_eq!(origin_of(""), None);
    }

    fn hdrs(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                header::HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn session_cookie_detection() {
        assert!(has_session_cookie(&hdrs(&[(
            "cookie",
            "__Host-session=abc; theme=dark"
        )])));
        assert!(has_session_cookie(&hdrs(&[("cookie", "session=abc")])));
        assert!(!has_session_cookie(&hdrs(&[("cookie", "theme=dark")])));
        // A cookie merely containing "session" in its name must not match.
        assert!(!has_session_cookie(&hdrs(&[("cookie", "mysession=x")])));
        assert!(!has_session_cookie(&hdrs(&[])));
    }
}
