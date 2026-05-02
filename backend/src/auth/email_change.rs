//! Email change: request issues a token to the *new* address; confirm
//! swaps it and notifies the *old* address.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::auth::password;
use crate::http::AppState;
use crate::mail::templates;

const TTL_HOURS: i32 = 1;
const PER_USER_COOLDOWN_SECS: f64 = 60.0;
const PER_HOUR_CAP: i64 = 5;

#[derive(Deserialize)]
pub struct RequestBody {
    pub new_email: String,
    pub current_password: String,
}

#[derive(Serialize)]
pub struct ConfirmResponse {
    pub status: String, // "success" | "expired" | "taken"
}

pub async fn request(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    // Verify current password. OAuth-only users (no password_hash) cannot
    // use this flow — they should use "Set password" first.
    let pwd_hash = user
        .password_hash
        .ok_or_else(|| AppError::bad_request("no_password_set"))?;
    let ok = password::verify(body.current_password, pwd_hash).await?;
    if !ok {
        return Err(AppError::Unauthorized);
    }

    let new_email = body.new_email.trim().to_lowercase();
    if new_email == user.email.to_lowercase() {
        return Err(AppError::bad_request("same_email"));
    }
    if !new_email.contains('@') {
        return Err(AppError::bad_request("invalid_email"));
    }

    // Throttle: check cooldown and hourly cap.
    let cooldown_hit = sqlx::query_scalar!(
        "select exists(
            select 1 from email_change_tokens
             where user_id = $1
               and created_at > now() - make_interval(secs => $2)
        )",
        user.id,
        PER_USER_COOLDOWN_SECS
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    let hour_cap_hit = sqlx::query_scalar!(
        "select count(*) >= $2 from email_change_tokens
          where user_id = $1
            and created_at > now() - interval '1 hour'",
        user.id,
        PER_HOUR_CAP
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    if cooldown_hit || hour_cap_hit {
        return Err(AppError::too_many_requests("email_change_throttled"));
    }

    // Invalidate any prior pending token for this user.
    sqlx::query!(
        "update email_change_tokens set used_at = now()
          where user_id = $1 and used_at is null",
        user.id
    )
    .execute(&state.pool)
    .await?;

    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

    sqlx::query!(
        "insert into email_change_tokens (token_hash, user_id, new_email, expires_at)
          values ($1, $2, $3, now() + make_interval(hours => $4))",
        hash,
        user.id,
        new_email,
        TTL_HOURS
    )
    .execute(&state.pool)
    .await?;

    let link = format!(
        "{}/email-change/{}",
        state.config.public_base_url.trim_end_matches('/'),
        token
    );
    let (subject, body) = templates::email_change_request(&user.email, &link);
    state.mailer.send_plain(&new_email, &subject, &body).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ConfirmBody {
    pub token: String,
}

pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmBody>,
) -> Result<Json<ConfirmResponse>, AppError> {
    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query!(
        r#"select user_id, new_email as "new_email!: String", expires_at, used_at
             from email_change_tokens
            where token_hash = $1
              for update"#,
        hash
    )
    .fetch_optional(&mut *tx)
    .await?;

    let row = match row {
        Some(r) if r.used_at.is_none() && r.expires_at > chrono::Utc::now() => r,
        _ => {
            tx.rollback().await.ok();
            return Ok(Json(ConfirmResponse {
                status: "expired".into(),
            }));
        }
    };

    let old = sqlx::query!(
        "select email as \"email!: String\" from users where id = $1",
        row.user_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let updated = sqlx::query!(
        "update users set email = $1 where id = $2",
        row.new_email,
        row.user_id
    )
    .execute(&mut *tx)
    .await;

    match updated {
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            tx.rollback().await.ok();
            return Ok(Json(ConfirmResponse {
                status: "taken".into(),
            }));
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    sqlx::query!(
        "update email_change_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let masked = templates::mask_email(&row.new_email);
    let when = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let (subject, notification_body) = templates::email_change_notification(&masked, &when);
    if let Err(e) = state
        .mailer
        .send_plain(&old.email, &subject, &notification_body)
        .await
    {
        tracing::warn!(error = ?e, user_id = %row.user_id,
            "email-change notification to old address failed; change still committed");
    }

    Ok(Json(ConfirmResponse {
        status: "success".into(),
    }))
}
