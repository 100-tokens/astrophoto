use axum::response::IntoResponse;

use crate::error::AppError;

pub async fn handler() -> Result<impl IntoResponse, AppError> {
    Err::<(), _>(AppError::Validation("not yet implemented".into()))
}
