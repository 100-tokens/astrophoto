//! Per-kind validation and INSERT helpers for `EquipmentSpecsPayload`.
//!
//! The DB check constraints (in migration 0018) are the hard rules.
//! This layer catches mismatched (kind, payload) pairs before the SQL
//! and centralises the per-sub-table INSERT, called by both
//! `items_create` (after a fresh or resolved equipment_items row) and
//! `items_update` (after a delete of the existing sub-table row).

use sqlx::types::BigDecimal;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::api_types::{
    CameraSpecs, EquipmentSpecsPayload, FilterSpecs, FocalModifierSpecs, MountSpecs, TelescopeSpecs,
};
use crate::error::AppError;

/// Verify the payload's tag matches the catalog item's `kind`.
/// Returns 422 on mismatch.
pub fn ensure_matches_kind(kind: &str, payload: &EquipmentSpecsPayload) -> Result<(), AppError> {
    let ok = matches!(
        (kind, payload),
        ("telescope", EquipmentSpecsPayload::Telescope(_))
            | ("camera", EquipmentSpecsPayload::Camera(_))
            | ("filter", EquipmentSpecsPayload::Filter(_))
            | ("mount", EquipmentSpecsPayload::Mount(_))
            | ("focal_modifier", EquipmentSpecsPayload::FocalModifier(_))
    );
    if ok {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "specs payload kind does not match item kind '{kind}'"
        )))
    }
}

/// Delete any existing `<kind>_specs` row for `item_id`.
/// Called before `insert_specs_row` so the INSERT doesn't hit a PK clash.
/// Silently no-ops if no row exists.
pub async fn delete_specs_row(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    kind: &str,
) -> Result<(), AppError> {
    match kind {
        "telescope" => {
            sqlx::query!("delete from telescope_specs where item_id = $1", item_id)
                .execute(&mut **tx)
                .await?;
        }
        "camera" => {
            sqlx::query!("delete from camera_specs where item_id = $1", item_id)
                .execute(&mut **tx)
                .await?;
        }
        "filter" => {
            sqlx::query!("delete from filter_specs where item_id = $1", item_id)
                .execute(&mut **tx)
                .await?;
        }
        "mount" => {
            sqlx::query!("delete from mount_specs where item_id = $1", item_id)
                .execute(&mut **tx)
                .await?;
        }
        "focal_modifier" => {
            sqlx::query!(
                "delete from focal_modifier_specs where item_id = $1",
                item_id
            )
            .execute(&mut **tx)
            .await?;
        }
        other => {
            return Err(AppError::Validation(format!(
                "unknown kind '{other}' in delete_specs_row"
            )));
        }
    }
    Ok(())
}

/// Insert a fresh sub-table row matching the payload's variant.
/// Callers MUST have ensured no prior row exists (PK on `item_id`).
pub async fn insert_specs_row(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    payload: &EquipmentSpecsPayload,
) -> Result<(), AppError> {
    match payload {
        EquipmentSpecsPayload::Telescope(s) => insert_telescope(tx, item_id, s).await,
        EquipmentSpecsPayload::Camera(s) => insert_camera(tx, item_id, s).await,
        EquipmentSpecsPayload::Filter(s) => insert_filter(tx, item_id, s).await,
        EquipmentSpecsPayload::Mount(s) => insert_mount(tx, item_id, s).await,
        EquipmentSpecsPayload::FocalModifier(s) => insert_focal_modifier(tx, item_id, s).await,
    }
}

fn enum_to_text<T: serde::Serialize>(v: &Option<T>) -> Option<String> {
    v.as_ref()
        .and_then(|e| serde_json::to_value(e).ok())
        .and_then(|val| val.as_str().map(String::from))
}

fn f64_to_decimal(v: Option<f64>) -> Option<BigDecimal> {
    use std::str::FromStr;
    v.and_then(|f| BigDecimal::from_str(&format!("{f}")).ok())
}

async fn insert_telescope(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &TelescopeSpecs,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into telescope_specs (item_id, design, aperture_mm, focal_length_mm)
            values ($1, $2, $3, $4)"#,
        item_id,
        enum_to_text(&s.design),
        s.aperture_mm,
        s.focal_length_mm,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_camera(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &CameraSpecs,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into camera_specs (
                item_id, sensor_type, color_type, cooled, sensor_model,
                pixel_size_um, sensor_width_px, sensor_height_px
            ) values ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        item_id,
        enum_to_text(&s.sensor_type),
        enum_to_text(&s.color_type),
        s.cooled,
        s.sensor_model.as_deref(),
        f64_to_decimal(s.pixel_size_um),
        s.sensor_width_px,
        s.sensor_height_px,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_filter(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &FilterSpecs,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into filter_specs (item_id, filter_type, bandwidth_nm, size, mounted)
            values ($1, $2, $3, $4, $5)"#,
        item_id,
        enum_to_text(&s.filter_type),
        f64_to_decimal(s.bandwidth_nm),
        enum_to_text(&s.size),
        s.mounted,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_mount(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &MountSpecs,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into mount_specs (item_id, mount_type, payload_kg, goto)
            values ($1, $2, $3, $4)"#,
        item_id,
        enum_to_text(&s.mount_type),
        f64_to_decimal(s.payload_kg),
        s.goto,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_focal_modifier(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &FocalModifierSpecs,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into focal_modifier_specs (item_id, modifier_type, factor)
            values ($1, $2, $3)"#,
        item_id,
        enum_to_text(&s.modifier_type),
        f64_to_decimal(s.factor),
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_types::{EquipmentSpecsPayload, FilterSpecs, TelescopeSpecs};

    #[test]
    fn rejects_payload_of_wrong_kind_for_item() {
        let payload = EquipmentSpecsPayload::Filter(FilterSpecs::default());
        let err = ensure_matches_kind("telescope", &payload).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn accepts_matching_kind() {
        let payload = EquipmentSpecsPayload::Telescope(TelescopeSpecs::default());
        ensure_matches_kind("telescope", &payload).unwrap();
    }

    #[test]
    fn enum_to_text_round_trip() {
        use crate::api_types::TelescopeDesign;
        let s = TelescopeSpecs {
            design: Some(TelescopeDesign::Newtonian),
            ..Default::default()
        };
        assert_eq!(enum_to_text(&s.design).as_deref(), Some("newtonian"));
    }
}
