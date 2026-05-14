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

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(SignupResponse {
            status: "verification_required",
            email: user.email,
        }),
    ))
}
