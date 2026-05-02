//! In-settings password change. Requires session + (current password OR
//! none, if the user is OAuth-only and has no password yet). Always
//! deletes every existing session and issues a fresh one (rotation).

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::auth::{password, session};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body {
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn change(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    password::validate_strength(&body.new_password).map_err(AppError::bad_request)?;

    // Verify current password only if the user actually has one.
    if let Some(stored_hash) = user.password_hash {
        let current = body.current_password.ok_or(AppError::Unauthorized)?;
        let ok = password::verify(current, stored_hash).await?;
        if !ok {
            return Err(AppError::Unauthorized);
        }
    }
    // OAuth-only path: no current_password expected.

    let pwd_hash = password::hash(body.new_password).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update users set password_hash = $1, password_changed_at = now() where id = $2",
        pwd_hash,
        user.id
    )
    .execute(&mut *tx)
    .await?;
    // Pure rotation: kill EVERY session including the current one, then
    // issue a fresh one for this browser. The browser keeps working
    // through the new cookie; other devices are signed out.
    sqlx::query!("delete from sessions where user_id = $1", user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let cookie = session::create_session(&state, user.id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|_| AppError::internal("bad cookie"))?,
    );
    Ok((StatusCode::NO_CONTENT, headers))
}
