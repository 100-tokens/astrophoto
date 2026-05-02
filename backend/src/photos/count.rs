use axum::{Json, extract::State};
use serde::Serialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::queries;

#[derive(Serialize)]
pub struct CountResponse {
    pub count: i64,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<CountResponse>, AppError> {
    let count = queries::count_by_owner(&state.pool, user.id).await?;
    Ok(Json(CountResponse { count }))
}
