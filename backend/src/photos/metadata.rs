use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Deserialize, Default)]
pub struct MetadataUpdate {
    pub target: Option<Option<String>>,
    pub caption: Option<Option<String>>,
    pub taken_at: Option<Option<DateTime<Utc>>>,
    pub camera: Option<Option<String>>,
    pub lens: Option<Option<String>>,
    pub iso: Option<Option<i32>>,
    pub exposure_s: Option<Option<f64>>,
    pub focal_mm: Option<Option<f64>>,
    pub exif_json: Option<serde_json::Value>,
    pub last_step: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(patch): Json<MetadataUpdate>,
) -> Result<StatusCode, AppError> {
    let owner = sqlx::query_scalar!("select owner_id from photos where id = $1", id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::not_found("photo"))?;

    if owner != user.id {
        return Err(AppError::Forbidden);
    }

    if let Some(s) = &patch.last_step
        && !["upload", "verify", "caption"].contains(&s.as_str())
    {
        return Err(AppError::Validation(format!("bad last_step: {s}")));
    }

    sqlx::query!(
        r#"
        update photos set
          target       = case when $2::bool then $3 else target end,
          caption      = case when $4::bool then $5 else caption end,
          taken_at     = case when $6::bool then $7 else taken_at end,
          camera       = case when $8::bool then $9 else camera end,
          lens         = case when $10::bool then $11 else lens end,
          iso          = case when $12::bool then $13 else iso end,
          exposure_s   = case when $14::bool then $15 else exposure_s end,
          focal_mm     = case when $16::bool then $17 else focal_mm end,
          exif_json    = case when $18::bool then $19 else exif_json end,
          last_step    = coalesce($20, last_step)
        where id = $1
        "#,
        id,
        patch.target.is_some(),
        patch.target.flatten(),
        patch.caption.is_some(),
        patch.caption.flatten(),
        patch.taken_at.is_some(),
        patch.taken_at.flatten(),
        patch.camera.is_some(),
        patch.camera.flatten(),
        patch.lens.is_some(),
        patch.lens.flatten(),
        patch.iso.is_some(),
        patch.iso.flatten(),
        patch.exposure_s.is_some(),
        patch.exposure_s.flatten(),
        patch.focal_mm.is_some(),
        patch.focal_mm.flatten(),
        patch.exif_json.is_some(),
        patch.exif_json,
        patch.last_step.as_deref(),
    )
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::OK)
}
