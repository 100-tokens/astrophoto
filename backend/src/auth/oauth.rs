use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OauthState {
    pub csrf_token: String,
    pub pkce_verifier: String,
}

pub const OAUTH_STATE_COOKIE: &str = "__Host-oauth";
