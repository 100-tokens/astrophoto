use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::queries;
use crate::photos::get::PhotoDetail;

#[derive(Deserialize)]
pub struct ListQuery {
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub photos: Vec<PhotoDetail>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = match q.owner_id {
        Some(id) => queries::list_by_owner(&state.pool, id, limit).await?,
        None => queries::list_recent_public(&state.pool, limit).await?,
    };
    Ok(Json(ListResponse {
        photos: rows.into_iter().map(Into::into).collect(),
    }))
}
