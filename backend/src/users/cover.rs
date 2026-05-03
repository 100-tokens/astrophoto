use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body {
    pub photo_id: Option<Uuid>,
}

pub async fn set(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(pid) = body.photo_id {
        // Photo must exist, be owned by the caller, be published, and be ready.
        let row = sqlx::query!(
            r#"
            select 1 as ok
            from photos
            where id = $1
              and owner_id = $2
              and published_at is not null
              and status = 'ready'
            "#,
            pid,
            user.id
        )
        .fetch_optional(&state.pool)
        .await?;
        if row.is_none() {
            return Err(AppError::not_found("photo_not_owned_or_unpublished"));
        }
        sqlx::query!(
            "update users set cover_photo_id = $1 where id = $2",
            pid,
            user.id
        )
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query!(
            "update users set cover_photo_id = null where id = $1",
            user.id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
