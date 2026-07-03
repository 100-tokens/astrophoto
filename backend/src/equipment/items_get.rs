//! GET /api/equipment/items/:id — item + joined specs.
//!
//! Returns `EquipmentItemDetail`. Specs are loaded from the per-kind
//! sub-table when present; rows with no matching sub-table row return
//! `specs: null`. Catalog v2 (migration 0022) adds the new
//! `guiding_specs` sub-table — `kind = 'guiding'` now joins through it.

use axum::{Json, extract::Path, extract::State, response::IntoResponse};
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use uuid::Uuid;

use crate::api_types::{
    CameraColorType, CameraSensorType, CameraSpecs, EquipmentItemDetail, EquipmentSpecsPayload,
    FilterSize, FilterSpecs, FilterType, FocalModifierSpecs, FocalModifierType, GuidingSetupKind,
    GuidingSpecs, MountSpecs, MountType, TelescopeDesign, TelescopeSpecs,
};
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Catalog v2 (browse phase): join the submitter handle so the
    // detail page's "added by @handle" footer works without a second
    // round-trip, and count distinct setups referencing this item so
    // the "Delete" affordance can hide when the item is in use.
    let row = sqlx::query!(
        r#"select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                  ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                  ei.brand, ei.model, ei.variant,
                  case when u.pending_deletion_at is null then u.handle end as "submitted_by_handle?",
                  (select count(distinct setup_id) from setup_items where item_id = ei.id)
                      as "setup_count!"
             from equipment_items ei
             left join users u on u.id = ei.submitted_by
            where ei.id = $1"#,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("equipment item not found".into()))?;

    let specs = match row.kind.as_str() {
        "telescope" => load_telescope(&state.pool, row.id).await?,
        "camera" => load_camera(&state.pool, row.id).await?,
        "filter" => load_filter(&state.pool, row.id).await?,
        "mount" => load_mount(&state.pool, row.id).await?,
        "focal_modifier" => load_focal_modifier(&state.pool, row.id).await?,
        "guiding" => load_guiding(&state.pool, row.id).await?,
        _ => None,
    };

    Ok(Json(EquipmentItemDetail {
        id: row.id.to_string(),
        kind: row.kind,
        canonical_name: row.canonical_name,
        display_name: row.display_name,
        usage_count: row.usage_count,
        status: row.status,
        submitted_by: row.submitted_by.map(|u| u.to_string()),
        approved_at: row.approved_at.map(|t| t.to_rfc3339()),
        created_at: row.created_at.to_rfc3339(),
        specs,
        brand: row.brand,
        model: row.model,
        variant: row.variant,
        submitted_by_handle: row.submitted_by_handle,
        setup_count: row.setup_count,
    }))
}

fn parse_enum<T>(s: Option<String>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    s.and_then(|val| serde_json::from_value(serde_json::Value::String(val)).ok())
}

fn decimal_to_f64(d: Option<BigDecimal>) -> Option<f64> {
    d.and_then(|x| x.to_string().parse::<f64>().ok())
}

async fn load_telescope(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select design, aperture_mm, focal_length_mm, focal_ratio_f,
                  self_weight_kg, optical_length_mm, backfocus_mm
             from telescope_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::Telescope(TelescopeSpecs {
            design: parse_enum::<TelescopeDesign>(r.design),
            aperture_mm: r.aperture_mm,
            focal_length_mm: r.focal_length_mm,
            focal_ratio_f: decimal_to_f64(r.focal_ratio_f),
            self_weight_kg: decimal_to_f64(r.self_weight_kg),
            optical_length_mm: r.optical_length_mm,
            backfocus_mm: decimal_to_f64(r.backfocus_mm),
        })
    }))
}

async fn load_camera(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select sensor_type, color_type, cooled, sensor_model,
                  pixel_size_um, sensor_width_px, sensor_height_px,
                  self_weight_g, full_well_capacity_e, read_noise_e,
                  mount_thread, backfocus_mm
             from camera_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::Camera(CameraSpecs {
            sensor_type: parse_enum::<CameraSensorType>(r.sensor_type),
            color_type: parse_enum::<CameraColorType>(r.color_type),
            cooled: r.cooled,
            sensor_model: r.sensor_model,
            pixel_size_um: decimal_to_f64(r.pixel_size_um),
            sensor_width_px: r.sensor_width_px,
            sensor_height_px: r.sensor_height_px,
            self_weight_g: r.self_weight_g,
            full_well_capacity_e: r.full_well_capacity_e,
            read_noise_e: decimal_to_f64(r.read_noise_e),
            mount_thread: r.mount_thread,
            backfocus_mm: decimal_to_f64(r.backfocus_mm),
        })
    }))
}

async fn load_filter(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select filter_type, bandwidth_nm, size, mounted,
                  mounted_diameter_mm, thickness_mm, peak_transmission_pct
             from filter_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::Filter(FilterSpecs {
            filter_type: parse_enum::<FilterType>(r.filter_type),
            bandwidth_nm: decimal_to_f64(r.bandwidth_nm),
            size: parse_enum::<FilterSize>(r.size),
            mounted: r.mounted,
            mounted_diameter_mm: decimal_to_f64(r.mounted_diameter_mm),
            thickness_mm: decimal_to_f64(r.thickness_mm),
            peak_transmission_pct: decimal_to_f64(r.peak_transmission_pct),
        })
    }))
}

async fn load_mount(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select mount_type, payload_kg, goto,
                  self_weight_kg, periodic_error_arcsec, tripod_included, control_protocol
             from mount_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::Mount(MountSpecs {
            mount_type: parse_enum::<MountType>(r.mount_type),
            payload_kg: decimal_to_f64(r.payload_kg),
            goto: r.goto,
            self_weight_kg: decimal_to_f64(r.self_weight_kg),
            periodic_error_arcsec: decimal_to_f64(r.periodic_error_arcsec),
            tripod_included: r.tripod_included,
            control_protocol: r.control_protocol,
        })
    }))
}

async fn load_focal_modifier(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select modifier_type, factor,
                  self_weight_g, backfocus_mm, image_circle_mm
             from focal_modifier_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::FocalModifier(FocalModifierSpecs {
            modifier_type: parse_enum::<FocalModifierType>(r.modifier_type),
            factor: decimal_to_f64(r.factor),
            self_weight_g: r.self_weight_g,
            backfocus_mm: decimal_to_f64(r.backfocus_mm),
            image_circle_mm: decimal_to_f64(r.image_circle_mm),
        })
    }))
}

async fn load_guiding(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select setup_kind, guide_focal_mm, guide_aperture_mm, guide_camera
             from guiding_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::Guiding(GuidingSpecs {
            // setup_kind is NOT NULL in the DB, but parse_enum is the same
            // path the other specs use — wrap and unwrap_or fallback to
            // Other so a future enum-value drift can't crash the GET.
            setup_kind: parse_enum::<GuidingSetupKind>(Some(r.setup_kind)),
            guide_focal_mm: r.guide_focal_mm,
            guide_aperture_mm: r.guide_aperture_mm,
            guide_camera: r.guide_camera,
        })
    }))
}
