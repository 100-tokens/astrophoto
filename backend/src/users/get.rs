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
    let user = user_q::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let count = photo_q::count_by_owner(&state.pool, id).await?;
    Ok(Json(UserPublic {
        id: user.id.to_string(),
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        photo_count: count,
    }))
}
