use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::AppError;
use crate::api_types::UserPublic;
use crate::http::AppState;
use crate::photos::queries as photo_q;
use crate::users::queries as user_q;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserPublic>, AppError> {
    // Grace-period accounts are delisted from public view. Do NOT push
    // this filter into find_by_id — the session middleware shares it,
    // and filtering there would log pending users out mid-grace.
    let visible: bool = sqlx::query_scalar!(
        r#"select (pending_deletion_at is null) as "v!" from users where id = $1"#,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .unwrap_or(false);
    if !visible {
        return Err(AppError::not_found("user"));
    }
    let user = user_q::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::not_found("user"))?;
    let count = photo_q::count_by_owner(&state.pool, id).await?;
    Ok(Json(UserPublic {
        id: user.id.to_string(),
        handle: user.handle,
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        photo_count: count,
    }))
}
