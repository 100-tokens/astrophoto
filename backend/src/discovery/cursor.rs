//! Shared opaque cursor for cross-author discovery feeds.
//! Same shape as `users::photos_feed::Cursor` (P2). Three-tuple
//! comparison `(appreciations_count, published_at, id)` powers the
//! `most-appreciated` sort; two-tuple `(published_at, id)` powers
//! `newest`. The `appreciations` field is `None` for newest.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use uuid::Uuid;

use crate::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub id: Uuid,
    #[serde(default)]
    pub appreciations: Option<i32>,
}

pub fn encode(c: &Cursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn decode(s: &str) -> Result<Cursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AppError::bad_request("cursor_invalid"))?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::bad_request("cursor_invalid"))
}
