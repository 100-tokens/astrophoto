//! Email verification: token issuance + verify endpoint + resend endpoint.
//! Mirrors the structure of `auth/password_reset.rs` exactly.

use std::net::{IpAddr, SocketAddr};

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
use sqlx::PgPool;
use sqlx::types::ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::AppError;
use crate::auth::session;
use crate::http::AppState;

pub(crate) const TTL_HOURS: i32 = 24;
pub(crate) const PER_EMAIL_COOLDOWN_SECS: f64 = 60.0;
pub(crate) const PER_HOUR_CAP: i64 = 5;
// Site-wide ceiling on verification mail per hour. See the matching constant in
// password_reset.rs for the rationale (the per-account cap is keyed on user_id
// alone now; this is a separate, non-OR-combined aggregate backstop).
const GLOBAL_HOUR_CAP: i64 = 200;

/// Generate a fresh token, insert its sha256 hash, return the raw token
/// (URL-safe base64, no padding) for embedding into the email link.
pub(crate) async fn issue_token(
    pool: &PgPool,
    user_id: Uuid,
    ip: Option<IpAddr>,
) -> Result<String, AppError> {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

    sqlx::query!(
        "insert into email_verification_tokens (token_hash, user_id, expires_at, request_ip)
          values ($1, $2, now() + make_interval(hours => $3), $4)",
        hash,
        user_id,
        TTL_HOURS,
        ip.map(IpNetwork::from)
    )
    .execute(pool)
    .await?;

    Ok(token)
}

#[derive(Deserialize)]
pub struct VerifyBody {
    pub token: String,
}

pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyBody>,
) -> Result<impl IntoResponse, AppError> {
    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"select user_id, expires_at, used_at
             from email_verification_tokens
            where token_hash = $1"#,
        hash
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::gone("expired_or_used"))?;

    if row.used_at.is_some() || row.expires_at < chrono::Utc::now() {
        return Err(AppError::gone("expired_or_used"));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update email_verification_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "update users set email_verified_at = now() where id = $1",
        row.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let cookie = session::create_session(&state, row.user_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|_| AppError::internal("bad cookie"))?,
    );
    Ok((StatusCode::OK, headers))
}

#[derive(Deserialize)]
pub struct ResendBody {
    pub email: String,
}

pub async fn resend(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<ResendBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"select id, email as "email!: String", display_name, email_verified_at
             from users where email = $1"#,
        body.email
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(u) = user
        && u.email_verified_at.is_none()
    {
        let cooldown_hit = sqlx::query_scalar!(
            "select exists(
                select 1 from email_verification_tokens
                where user_id = $1
                  and created_at > now() - make_interval(secs => $2)
            )",
            u.id,
            PER_EMAIL_COOLDOWN_SECS
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        // Per-account hourly cap, keyed on user_id ONLY (no longer OR'd with the
        // proxy request_ip — that collapsed it into a single global bucket).
        let hour_cap_hit = sqlx::query_scalar!(
            "select count(*) >= $2 from email_verification_tokens
              where user_id = $1
                and created_at > now() - interval '1 hour'",
            u.id,
            PER_HOUR_CAP
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        // Separate site-wide ceiling on outbound verification mail.
        let global_cap_hit = sqlx::query_scalar!(
            "select count(*) >= $1 from email_verification_tokens
              where created_at > now() - interval '1 hour'",
            GLOBAL_HOUR_CAP
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);
        if global_cap_hit {
            tracing::warn!("email-verify aggregate hourly cap hit; suppressing token issuance");
        }

        if !cooldown_hit && !hour_cap_hit && !global_cap_hit {
            let token = issue_token(&state.pool, u.id, Some(addr.ip())).await?;
            let link = format!(
                "{}/verify/{}",
                state.config.public_base_url.trim_end_matches('/'),
                token
            );
            let (subject, mail_body) =
                crate::mail::templates::email_verification(&u.display_name, &link);
            if let Err(e) = state
                .mailer
                .send_plain(&u.email, &subject, &mail_body)
                .await
            {
                tracing::warn!(error = %e, user_id = %u.id, "email-verification mail send failed");
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::queries::create_with_password;

    #[tokio::test]
    async fn issue_token_writes_a_row_we_can_find_by_hash() {
        let pg = testcontainers::runners::AsyncRunner::start(testcontainers::ImageExt::with_tag(
            testcontainers_modules::postgres::Postgres::default(),
            "16-alpine",
        ))
        .await
        .unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let user = create_with_password(&pool, "tok@example.com", "tok-abc", "T", "hash")
            .await
            .unwrap();
        let token = issue_token(&pool, user.id, None).await.unwrap();
        assert_eq!(token.len(), 43); // 32 bytes -> 43 chars URL-safe base64 (no pad)

        let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
        let row = sqlx::query!(
            "select user_id, used_at, expires_at from email_verification_tokens where token_hash = $1",
            hash
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.user_id, user.id);
        assert!(row.used_at.is_none());
        assert!(row.expires_at > chrono::Utc::now());
    }
}
