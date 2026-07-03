use axum::{
    Json,
    extract::{Path, State},
};

use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(old_handle): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query!(
        r#"select u.handle::text as "handle!"
             from handle_redirects r
             join users u on u.id = r.user_id
            where r.old_handle = $1 and u.pending_deletion_at is null"#,
        old_handle
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("redirect"))?;

    Ok(Json(serde_json::json!({ "handle": row.handle })))
}
