//! GET /api/equipment/items/:id — item + joined specs.
//!
//! Returns `EquipmentItemDetail`. Specs are loaded from the per-kind
//! sub-table when present; `kind = 'guiding'` (legacy) returns
//! `specs: null` since no sub-table exists for it.

use axum::{Json, extract::Path, extract::State, response::IntoResponse};
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use uuid::Uuid;

use crate::api_types::{
    CameraColorType, CameraSensorType, CameraSpecs, EquipmentItemDetail, EquipmentSpecsPayload,
    FilterSize, FilterSpecs, FilterType, FocalModifierSpecs, FocalModifierType, MountSpecs,
    MountType, TelescopeDesign, TelescopeSpecs,
};
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"select id, kind, canonical_name, display_name, usage_count,
                  status, submitted_by, approved_at, created_at
             from equipment_items where id = $1"#,
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
        r#"select design, aperture_mm, focal_length_mm, focal_ratio_f
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
        })
    }))
}

async fn load_camera(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select sensor_type, color_type, cooled, sensor_model,
                  pixel_size_um, sensor_width_px, sensor_height_px
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
        })
    }))
}

async fn load_filter(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select filter_type, bandwidth_nm, size, mounted
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
        })
    }))
}

async fn load_mount(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select mount_type, payload_kg, goto
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
        })
    }))
}

async fn load_focal_modifier(
    pool: &PgPool,
    item_id: Uuid,
) -> Result<Option<EquipmentSpecsPayload>, AppError> {
    let r = sqlx::query!(
        r#"select modifier_type, factor
             from focal_modifier_specs where item_id = $1"#,
        item_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map(|r| {
        EquipmentSpecsPayload::FocalModifier(FocalModifierSpecs {
            modifier_type: parse_enum::<FocalModifierType>(r.modifier_type),
            factor: decimal_to_f64(r.factor),
        })
    }))
}
