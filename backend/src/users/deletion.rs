use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::auth::middleware::{CurrentUser, SessionOnly};
use crate::http::AppState;
use crate::mail::templates;

#[derive(Deserialize)]
pub struct RequestBody {
    pub current_password: Option<String>,
    pub confirmation_phrase: String,
}

const REQUIRED_PHRASE: &str = "DELETE MY ACCOUNT";

pub async fn request(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    // Account-control endpoint: browser sessions only, never PATs.
    _session_only: SessionOnly,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.confirmation_phrase != REQUIRED_PHRASE {
        return Err(AppError::bad_request("phrase_mismatch"));
    }

    let row = sqlx::query!(
        r#"select email as "email!: String", display_name, password_hash
           from users where id = $1"#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    if let Some(stored) = row.password_hash {
        let pwd = body.current_password.ok_or(AppError::Unauthorized)?;
        let ok = crate::auth::password::verify(pwd, stored).await?;
        if !ok {
            return Err(AppError::Unauthorized);
        }
    }

    let updated = sqlx::query!(
        r#"update users set pending_deletion_at = now() + interval '7 days'
          where id = $1 and pending_deletion_at is null
        returning pending_deletion_at as "pending_deletion_at!: chrono::DateTime<chrono::Utc>""#,
        user.id
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(u) = updated {
        let when_human = u
            .pending_deletion_at
            .format("%A %e %B %Y at %H:%M UTC")
            .to_string();
        let cancel_link = format!(
            "{}/settings/delete",
            state.config.public_base_url.trim_end_matches('/')
        );
        let (subject, body) =
            templates::account_deletion_scheduled(&row.display_name, &when_human, &cancel_link);
        if let Err(e) = state.mailer.send_plain(&row.email, &subject, &body).await {
            tracing::warn!(error = ?e, user_id = %user.id,
                "account-deletion scheduled notification failed; state still committed");
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    // Account-control endpoint: browser sessions only, never PATs.
    _session_only: SessionOnly,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"update users set pending_deletion_at = null
          where id = $1 and pending_deletion_at is not null
        returning email as "email!: String", display_name"#,
        user.id
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(r) = row {
        let (subject, body) = templates::account_deletion_cancelled(&r.display_name);
        if let Err(e) = state.mailer.send_plain(&r.email, &subject, &body).await {
            tracing::warn!(error = ?e, user_id = %user.id,
                "account-deletion cancellation notification failed; state still committed");
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
