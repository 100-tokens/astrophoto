//! Personal access tokens ("PAT") for native clients (PixInsight
//! plugin). Secrets look like `astrophoto_pat_<43 url-safe chars>`;
//! only their SHA-256 is persisted.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{ApiTokenCreated, ApiTokenRow};
use crate::auth::middleware::{CurrentUser, TokenAuth};
use crate::http::AppState;

pub const TOKEN_PREFIX: &str = "astrophoto_pat_";

/// Length of the displayable prefix stored alongside the hash.
const DISPLAY_PREFIX_LEN: usize = 20; // "astrophoto_pat_" + 5 chars

pub fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "{TOKEN_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

pub fn hash_secret(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

pub fn display_prefix(secret: &str) -> &str {
    &secret[..DISPLAY_PREFIX_LEN]
}

#[derive(serde::Deserialize)]
pub struct CreateBody {
    pub name: String,
}

/// Guard shared by the management handlers: a PAT must not mint or
/// revoke PATs — that stays a logged-in-browser operation.
fn reject_token_auth(parts_has_token: bool) -> Result<(), AppError> {
    if parts_has_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// `POST /api/me/tokens` — mint a new personal access token. The full
/// secret is returned exactly once in the response body.
///
/// **Ordering invariant:** `token_auth` reads request extensions that
/// `CurrentUser`'s `resolve()` populates, so it MUST appear after
/// `CurrentUser` in the signature (see `CurrentSessionId`).
pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    token_auth: Option<axum::Extension<TokenAuth>>,
    Json(body): Json<CreateBody>,
) -> Result<Json<ApiTokenCreated>, AppError> {
    reject_token_auth(token_auth.is_some())?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(AppError::bad_request("name"));
    }
    let secret = generate_secret();
    let hash = hash_secret(&secret);
    let prefix = display_prefix(&secret).to_string();
    let row = sqlx::query!(
        r#"insert into api_tokens (user_id, name, token_hash, prefix)
           values ($1, $2, $3, $4)
           returning id"#,
        user.id,
        name,
        hash,
        prefix
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(ApiTokenCreated {
        id: row.id.to_string(),
        name: name.to_string(),
        prefix,
        secret,
    }))
}

/// `GET /api/me/tokens` — list the caller's tokens, newest first.
pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<ApiTokenRow>>, AppError> {
    let rows = sqlx::query!(
        r#"select id, name, prefix, created_at, last_used_at, revoked_at
             from api_tokens where user_id = $1
            order by created_at desc"#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ApiTokenRow {
                id: r.id.to_string(),
                name: r.name,
                prefix: r.prefix,
                created_at: r.created_at.to_rfc3339(),
                last_used_at: r.last_used_at.map(|t| t.to_rfc3339()),
                revoked_at: r.revoked_at.map(|t| t.to_rfc3339()),
            })
            .collect(),
    ))
}

/// `DELETE /api/me/tokens/:id` — revoke one of the caller's tokens.
///
/// **Ordering invariant:** `token_auth` must follow `CurrentUser` (see
/// `create`).
pub async fn revoke(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    token_auth: Option<axum::Extension<TokenAuth>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    reject_token_auth(token_auth.is_some())?;
    let res = sqlx::query!(
        "update api_tokens set revoked_at = now() \
         where id = $1 and user_id = $2 and revoked_at is null",
        id,
        user.id
    )
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("token"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_has_prefix_and_length() {
        let s = generate_secret();
        assert!(s.starts_with(TOKEN_PREFIX));
        // 15-char prefix + 43 base64url chars for 32 bytes
        assert_eq!(s.len(), TOKEN_PREFIX.len() + 43);
    }

    #[test]
    fn hash_is_stable_and_32_bytes() {
        let s = generate_secret();
        assert_eq!(hash_secret(&s), hash_secret(&s));
        assert_eq!(hash_secret(&s).len(), 32);
    }

    #[test]
    fn two_secrets_differ() {
        assert_ne!(generate_secret(), generate_secret());
    }
}
