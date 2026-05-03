use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::auth::handle::{HandleError, validate};
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub handle: String,
}

#[derive(Serialize)]
pub struct R {
    pub status: &'static str,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    match validate(&q.handle) {
        Err(HandleError::Format) => return Ok(Json(R { status: "invalid" })),
        Err(HandleError::Reserved) => return Ok(Json(R { status: "reserved" })),
        Ok(()) => {}
    }

    let taken: bool = sqlx::query_scalar!(
        "select exists(select 1 from users where handle = $1)",
        q.handle
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    Ok(Json(R {
        status: if taken { "taken" } else { "available" },
    }))
}
