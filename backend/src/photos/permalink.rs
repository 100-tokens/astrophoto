use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::error::AppError;
use crate::http::AppState;

pub async fn lookup(
    State(state): State<AppState>,
    Path((handle, short_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"
        select p.id as "id!"
          from photos p
          join users  u on u.id = p.owner_id
         where u.handle   = $1
           and p.short_id  = $2
           and p.published_at is not null
        "#,
        handle,
        short_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    Ok(Json(serde_json::json!({ "id": row.id })))
}
