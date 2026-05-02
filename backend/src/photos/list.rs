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
    pub drafts: Option<bool>,
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

    if q.drafts.unwrap_or(false) {
        let me = user.0.ok_or(AppError::Unauthorized)?;
        // Reject cross-user drafts — users can only ever see their own.
        if let Some(requested) = q.owner_id {
            if requested != me.id {
                return Err(AppError::Forbidden);
            }
        }
        let rows = queries::list_drafts_by_owner(&state.pool, me.id, limit).await?;
        return Ok(Json(ListResponse {
            photos: rows.into_iter().map(Into::into).collect(),
        }));
    }

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
