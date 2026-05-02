use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::get::PhotoDetail;
use crate::photos::queries;

#[derive(Deserialize)]
pub struct ListQuery {
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub following: Option<bool>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub photos: Vec<PhotoDetail>,
}

pub async fn handler(
    State(state): State<AppState>,
    user: crate::auth::middleware::OptionalUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let rows = if q.following.unwrap_or(false) {
        let follower = user.0.ok_or(AppError::Unauthorized)?;
        queries::list_following(&state.pool, follower.id, limit).await?
    } else if let Some(id) = q.owner_id {
        queries::list_by_owner(&state.pool, id, limit).await?
    } else {
        queries::list_recent_public(&state.pool, limit).await?
    };

    Ok(Json(ListResponse {
        photos: rows.into_iter().map(Into::into).collect(),
    }))
}
