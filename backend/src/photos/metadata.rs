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
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub target: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub caption: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub taken_at: Option<Option<DateTime<Utc>>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub camera: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub lens: Option<Option<String>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub iso: Option<Option<i32>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub exposure_s: Option<Option<f64>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub focal_mm: Option<Option<f64>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub ra_deg: Option<Option<f64>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub dec_deg: Option<Option<f64>>,

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
          ra_deg       = case when $18::bool then $19 else ra_deg end,
          dec_deg      = case when $20::bool then $21 else dec_deg end,
          exif_json    = case when $22::bool then $23 else exif_json end,
          last_step    = coalesce($24, last_step)
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
        patch.ra_deg.is_some(),
        patch.ra_deg.flatten(),
        patch.dec_deg.is_some(),
        patch.dec_deg.flatten(),
        patch.exif_json.is_some(),
        patch.exif_json,
        patch.last_step.as_deref(),
    )
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::OK)
}
