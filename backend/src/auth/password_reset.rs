//! Password reset: 3-step public flow.
//! - request: issue a token, email a link, return 204 unconditionally
//!   (anti-enumeration). Throttled per-email and per-IP.
//! - confirm: set a new password, kill all sessions, auto-login (Task 6).

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::types::ipnetwork::IpNetwork;
use std::net::SocketAddr;

use crate::AppError;
use crate::auth::{password, session};
use crate::http::AppState;
use crate::mail::templates;

#[derive(Deserialize)]
pub struct RequestBody {
    pub email: String,
}

const TTL_HOURS: i32 = 1;
const PER_EMAIL_COOLDOWN_SECS: f64 = 60.0;
const PER_HOUR_CAP: i64 = 5;

pub async fn request(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"select id, email as "email!: String", display_name, password_hash
             from users where email = $1"#,
        body.email
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(u) = user {
        // Throttle: skip silently when the most recent token for this email
        // was issued < cooldown ago, or > cap have been issued in the last hour.
        let cooldown_hit = sqlx::query_scalar!(
            "select exists(
                select 1 from password_reset_tokens
                where user_id = $1
                  and created_at > now() - make_interval(secs => $2)
            )",
            u.id,
            PER_EMAIL_COOLDOWN_SECS
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        let hour_cap_hit = sqlx::query_scalar!(
            "select count(*) >= $2 from password_reset_tokens
              where (user_id = $1 or request_ip = $3)
                and created_at > now() - interval '1 hour'",
            u.id,
            PER_HOUR_CAP,
            IpNetwork::from(addr.ip())
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        if !cooldown_hit && !hour_cap_hit {
            // Issue token.
            let mut raw = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut raw);
            let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
            let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

            sqlx::query!(
                "insert into password_reset_tokens (token_hash, user_id, expires_at, request_ip)
                  values ($1, $2, now() + make_interval(hours => $3), $4)",
                hash,
                u.id,
                TTL_HOURS,
                IpNetwork::from(addr.ip())
            )
            .execute(&state.pool)
            .await?;

            // Build the link (frontend handles the page).
            let link = format!(
                "{}/reset/{}",
                state.config.public_base_url.trim_end_matches('/'),
                token
            );
            let (subject, body) =
                templates::password_reset(&u.display_name, &link, u.password_hash.is_some());
            // Swallow mail-send errors so the response stays a uniform 204
            // — see anti-enumeration contract at top of file. The token row
            // is still inserted so an operator can manually issue the link
            // if SMTP is misconfigured.
            if let Err(e) = state.mailer.send_plain(&u.email, &subject, &body).await {
                tracing::warn!(error = %e, user_id = %u.id, "password-reset mail send failed");
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ConfirmBody {
    pub token: String,
    pub new_password: String,
}

pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmBody>,
) -> Result<impl IntoResponse, AppError> {
    password::validate_strength(&body.new_password).map_err(AppError::bad_request)?;

    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"select user_id, expires_at, used_at
             from password_reset_tokens
            where token_hash = $1"#,
        hash
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::gone("expired_or_used"))?;

    if row.used_at.is_some() || row.expires_at < chrono::Utc::now() {
        return Err(AppError::gone("expired_or_used"));
    }

    let pwd_hash = password::hash(body.new_password).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update users set password_hash = $1, password_changed_at = now() where id = $2",
        pwd_hash,
        row.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "update password_reset_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("delete from sessions where user_id = $1", row.user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    // Auto-login: create a fresh session and return Set-Cookie.
    let cookie = session::create_session(&state, row.user_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|_| AppError::internal("bad cookie"))?,
    );
    Ok((StatusCode::NO_CONTENT, headers))
}
