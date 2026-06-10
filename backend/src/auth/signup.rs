use std::net::IpAddr;

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::AppError;
use crate::auth::email_verify;
use crate::mail::templates;
use crate::users::queries;

#[derive(Deserialize, Validate)]
pub struct SignupBody {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 10, max = 200))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub handle: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub status: &'static str,
    pub email: String,
}

pub async fn handler(
    State(state): State<crate::http::AppState>,
    headers: HeaderMap,
    Json(body): Json<SignupBody>,
) -> Result<impl IntoResponse, AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    crate::auth::handle::validate(&body.handle).map_err(|e| AppError::Validation(e.to_string()))?;

    // Runtime maintenance switch (super-admin setting). Checked before the
    // expensive password hash so a closed signup is cheap to reject. The
    // settings reader is fail-safe (defaults to enabled on any error).
    if !crate::settings::get(&state.pool).await.signups_enabled {
        return Err(AppError::BadRequest(
            "registration is currently closed".into(),
        ));
    }

    let hash = crate::auth::password::hash(body.password).await?;
    let user = queries::create_with_password(
        &state.pool,
        &body.email,
        &body.handle,
        &body.display_name,
        &hash,
    )
    .await?;

    let ip: Option<IpAddr> = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok());

    // Site-wide hourly ceiling on outbound verification mail, shared with
    // the resend endpoint (same token bucket). Without it, mass signups turn
    // the service into a mail-bombing relay and burn SES reputation. On cap
    // hit the account is still created and the response is unchanged — the
    // user can use "resend verification" once the window clears. Per-IP
    // throttling is intentionally absent (see login_throttle.rs: behind the
    // reverse proxy the IP axis collapses to one egress address).
    if email_verify::global_cap_hit(&state.pool).await? {
        tracing::warn!(
            user_id = %user.id,
            "signup: aggregate hourly verification-mail cap hit; suppressing token issuance"
        );
    } else {
        let token = email_verify::issue_token(&state.pool, user.id, ip).await?;
        let link = format!(
            "{}/verify/{}",
            state.config.public_base_url.trim_end_matches('/'),
            token
        );
        let (subject, mail_body) = templates::email_verification(&user.display_name, &link);
        if let Err(e) = state
            .mailer
            .send_plain(&user.email, &subject, &mail_body)
            .await
        {
            tracing::warn!(error = %e, user_id = %user.id, "signup verification mail send failed");
        }
    }

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(SignupResponse {
            status: "verification_required",
            email: user.email,
        }),
    ))
}
