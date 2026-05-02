use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::queries;

pub async fn thumb(
    State(state): State<AppState>,
    Path((id, size)): Path<(Uuid, i32)>,
) -> Result<Response, AppError> {
    if !matches!(size, 400 | 1200) {
        return Err(AppError::Validation("size must be 400 or 1200".into()));
    }
    let key = queries::thumb_storage_key(&state.pool, id, size)
        .await?
        .ok_or(AppError::NotFound)?;
    let bytes = state.storage.get(&key).await?.ok_or(AppError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok((headers, Body::from(bytes)).into_response())
}
