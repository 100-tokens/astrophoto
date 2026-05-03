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

    // Showcase Phase 1: new fields — simple Option (fill-in, not clear-able).
    pub category: Option<String>,
    pub scope: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
    pub tags: Option<Vec<String>>,
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

    if let Some(c) = &patch.category {
        if !matches!(
            c.as_str(),
            "dso" | "planetary" | "lunar" | "solar" | "wide_field" | "nightscape" | "other"
        ) {
            return Err(AppError::Validation("invalid category".into()));
        }
    }

    if let Some(tags) = &patch.tags
        && tags.len() > 8
    {
        return Err(AppError::Validation("max 8 tags".into()));
    }

    // Extract values needed after the UPDATE before moving them into the query.
    let target_freetext: Option<String> = patch.target.as_ref().and_then(|v| v.clone());
    let camera_freetext: Option<String> = patch.camera.as_ref().and_then(|v| v.clone());
    let scope_freetext = patch.scope.clone();
    let mount_freetext = patch.mount.clone();
    let filters_freetext = patch.filters.clone();
    let guiding_freetext = patch.guiding.clone();
    let tags_list = patch.tags.clone();

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
          last_step    = coalesce($24, last_step),
          category     = coalesce($25, category),
          scope        = coalesce($26, scope),
          mount        = coalesce($27, mount),
          filters      = coalesce($28, filters),
          guiding      = coalesce($29, guiding)
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
        patch.category.as_deref(),
        patch.scope.as_deref(),
        patch.mount.as_deref(),
        patch.filters.as_deref(),
        patch.guiding.as_deref(),
    )
    .execute(&state.pool)
    .await?;

    // --- post-update helpers (called after photos row is written) ---

    if let Some(target) = &target_freetext {
        crate::photos::targets::attach_primary_by_freetext(&state.pool, id, target).await?;
    }

    if let Some(tags) = &tags_list {
        crate::photos::tags::attach(&state.pool, id, tags).await?;
    }

    for (kind, val) in [
        ("camera", camera_freetext.as_deref()),
        ("telescope", scope_freetext.as_deref()),
        ("mount", mount_freetext.as_deref()),
        ("filter", filters_freetext.as_deref()),
        ("guiding", guiding_freetext.as_deref()),
    ] {
        if let Some(v) = val {
            if !v.trim().is_empty() {
                crate::equipment::upsert::upsert(&state.pool, kind, v).await?;
            }
        }
    }

    Ok(StatusCode::OK)
}
