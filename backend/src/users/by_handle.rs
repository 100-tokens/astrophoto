use axum::{
    Json,
    extract::{Path, State},
};

use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query!(
        r#"
        select id, handle::text as "handle!", display_name, created_at,
               (select count(*) from photos where owner_id = users.id and published_at is not null)
                   as "photo_count!"
          from users where handle = $1
        "#,
        handle
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("user"))?;

    Ok(Json(serde_json::json!({
        "id":           row.id,
        "handle":       row.handle,
        "display_name": row.display_name,
        "created_at":   row.created_at.to_rfc3339(),
        "photo_count":  row.photo_count,
    })))
}
