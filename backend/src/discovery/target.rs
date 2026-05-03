use axum::response::IntoResponse;
use crate::AppError;

pub async fn get() -> Result<impl IntoResponse, AppError> {
    Err::<&'static str, _>(AppError::internal("discovery_not_implemented"))
}
