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

    // Migration 0013: extended acquisition fields. Clearable (double-Option)
    // because users edit them via the verify form and need to remove a value.
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub aperture_f: Option<Option<f32>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub gain: Option<Option<i16>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub sensor_temp_c: Option<Option<f32>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    pub sessions: Option<Option<i16>>,

    pub exif_json: Option<serde_json::Value>,

    pub last_step: Option<String>,

    // Showcase Phase 1: new fields — simple Option (fill-in, not clear-able).
    pub category: Option<String>,
    pub scope: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
    pub tags: Option<Vec<String>>,
    // Per-photo focal modifier (fill-in, not clear-able via this route; migration 0017).
    pub focal_modifier: Option<String>,

    /// When present and non-null, replaces the manual target list atomically
    /// and SUPPRESSES the legacy attach_primary_by_freetext path for this request.
    /// `None` (omitted) → existing behaviour (free-text resolution from `target` field).
    /// `Some(vec)` → use this list; the `target` free-text field is ignored for join rows.
    #[serde(default)]
    pub targets: Option<Vec<String>>,

    /// Structured filter junction. When present, REPLACES the `photo_filters`
    /// junction atomically (positions = array index) and rebuilds the
    /// `photos.filters` cache string from the junction — overriding any
    /// legacy `filters` text field sent in the same request.
    /// `None` (omitted) → no change to the junction.
    #[serde(default)]
    pub filter_item_ids: Option<Vec<uuid::Uuid>>,
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

    if let Some(c) = &patch.category
        && !matches!(
            c.as_str(),
            "dso" | "planetary" | "lunar" | "solar" | "wide_field" | "nightscape" | "other"
        )
    {
        return Err(AppError::Validation("invalid category".into()));
    }

    if let Some(tags) = &patch.tags
        && tags.len() > 8
    {
        return Err(AppError::Validation("max 8 tags".into()));
    }

    // Extract values needed after the UPDATE before moving them into the query.
    let target_freetext: Option<String> = patch.target.as_ref().and_then(|v| v.clone());
    let targets_list = patch.targets.clone();
    let camera_freetext: Option<String> = patch.camera.as_ref().and_then(|v| v.clone());
    let scope_freetext = patch.scope.clone();
    let mount_freetext = patch.mount.clone();
    let filters_freetext = patch.filters.clone();
    let focal_modifier_freetext = patch.focal_modifier.clone();
    let tags_list = patch.tags.clone();
    let filter_item_ids = patch.filter_item_ids.clone();

    // Open a transaction so the photos UPDATE and the target join-table writes
    // are committed or rolled back together.
    let mut tx = state.pool.begin().await?;

    sqlx::query!(
        r#"
        update photos set
          target        = case when $2::bool  then $3  else target end,
          caption       = case when $4::bool  then $5  else caption end,
          taken_at      = case when $6::bool  then $7  else taken_at end,
          camera        = case when $8::bool  then $9  else camera end,
          lens          = case when $10::bool then $11 else lens end,
          iso           = case when $12::bool then $13 else iso end,
          exposure_s    = case when $14::bool then $15 else exposure_s end,
          focal_mm      = case when $16::bool then $17 else focal_mm end,
          ra_deg        = case when $18::bool then $19 else ra_deg end,
          dec_deg       = case when $20::bool then $21 else dec_deg end,
          exif_json     = case when $22::bool then $23 else exif_json end,
          aperture_f    = case when $30::bool then $31 else aperture_f end,
          gain          = case when $32::bool then $33 else gain end,
          sensor_temp_c = case when $34::bool then $35 else sensor_temp_c end,
          sessions      = case when $36::bool then $37 else sessions end,
          last_step      = coalesce($24, last_step),
          category       = coalesce($25, category),
          scope          = coalesce($26, scope),
          mount          = coalesce($27, mount),
          filters        = coalesce($28, filters),
          guiding        = coalesce($29, guiding),
          focal_modifier = coalesce($38, focal_modifier)
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
        patch.aperture_f.is_some(),
        patch.aperture_f.flatten(),
        patch.gain.is_some(),
        patch.gain.flatten(),
        patch.sensor_temp_c.is_some(),
        patch.sensor_temp_c.flatten(),
        patch.sessions.is_some(),
        patch.sessions.flatten(),
        patch.focal_modifier.as_deref(),
    )
    .execute(&mut *tx)
    .await?;

    // --- target join-table: new multi-slug path takes precedence ---
    if let Some(slugs) = &targets_list {
        crate::photos::targets::multi_attach(&mut tx, id, slugs).await?;
    } else if let Some(target) = &target_freetext {
        // Legacy free-text path: lenient (unknown slug → no-op).
        crate::photos::targets::attach_primary_by_freetext(&mut tx, id, target).await?;
    }

    // --- structured filter junction sync ---
    // When filter_item_ids is present, REPLACE the junction and rebuild the
    // cache string. This overrides any legacy `filters` text written above.
    if let Some(filter_ids) = &filter_item_ids {
        // Dedup while preserving first-seen order. PK on (photo_id, item_id)
        // would otherwise 500 if the same id appears twice.
        let mut seen = std::collections::HashSet::new();
        let unique_ids: Vec<uuid::Uuid> = filter_ids
            .iter()
            .filter(|id| seen.insert(**id))
            .copied()
            .collect();

        // Validate every id is kind='filter'.
        let count: i64 = if unique_ids.is_empty() {
            0
        } else {
            sqlx::query_scalar!(
                "select count(*) from equipment_items
                  where id = any($1) and kind = 'filter'",
                &unique_ids
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0)
        };
        if (count as usize) != unique_ids.len() {
            return Err(AppError::Validation(
                "filter_item_ids contains an unknown id or a non-filter kind".into(),
            ));
        }
        sqlx::query!("delete from photo_filters where photo_id = $1", id)
            .execute(&mut *tx)
            .await?;
        for (i, item_id) in unique_ids.iter().enumerate() {
            sqlx::query!(
                "insert into photo_filters (photo_id, item_id, position) values ($1, $2, $3)",
                id,
                item_id,
                i as i16
            )
            .execute(&mut *tx)
            .await?;
        }
        crate::photos::filters_cache::rebuild(&mut tx, id).await?;
    }

    tx.commit().await?;

    // --- remaining post-update helpers run outside the transaction ---

    if let Some(tags) = &tags_list {
        sqlx::query!("delete from photo_tags where photo_id = $1", id)
            .execute(&state.pool)
            .await?;
        crate::photos::tags::attach(&state.pool, id, tags).await?;
    }

    for (kind, val) in [
        ("camera", camera_freetext.as_deref()),
        ("telescope", scope_freetext.as_deref()),
        ("mount", mount_freetext.as_deref()),
        ("filter", filters_freetext.as_deref()),
        ("focal_modifier", focal_modifier_freetext.as_deref()),
    ] {
        if let Some(v) = val
            && !v.trim().is_empty()
        {
            crate::equipment::upsert::upsert(&state.pool, kind, v).await?;
        }
    }

    Ok(StatusCode::OK)
}
