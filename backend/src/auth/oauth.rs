use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OauthState {
    pub csrf_token: String,
    pub pkce_verifier: String,
}

/// Cookie name. The `__Host-` prefix requires HTTPS; in dev over plain HTTP the
/// browser silently rejects it, so we drop the prefix when `secure` is false.
pub fn oauth_cookie_name(secure: bool) -> &'static str {
    if secure {
        "__Host-oauth"
    } else {
        "oauth-state"
    }
}

/// Both names a request might carry on the read path.
pub const OAUTH_COOKIE_NAMES: &[&str] = &["__Host-oauth", "oauth-state"];
