//! Per-kind validation and INSERT helpers for `EquipmentSpecsPayload`.
//!
//! The DB check constraints (in migrations 0018 + 0022) are the hard
//! rules. This layer catches mismatched (kind, payload) pairs before
//! the SQL and centralises the per-sub-table INSERT, called by both
//! `items_create` (after a fresh or resolved equipment_items row) and
//! `items_update` (after a delete of the existing sub-table row).
//!
//! Catalog v2 (migration 0022): added Guiding variant + insert path,
//! plus extended each existing insert to write the new completeness
//! columns (self_weight_*, backfocus_mm, etc.).

use sqlx::types::BigDecimal;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::api_types::{
    CameraSpecs, EquipmentSpecsPayload, FilterSpecs, FocalModifierSpecs, GuidingSpecs, MountSpecs,
    TelescopeSpecs,
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
            | ("guiding", EquipmentSpecsPayload::Guiding(_))
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
        "guiding" => {
            sqlx::query!("delete from guiding_specs where item_id = $1", item_id)
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
        EquipmentSpecsPayload::Guiding(s) => insert_guiding(tx, item_id, s).await,
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
        r#"insert into telescope_specs
            (item_id, design, aperture_mm, focal_length_mm,
             self_weight_kg, optical_length_mm, backfocus_mm)
            values ($1, $2, $3, $4, $5, $6, $7)"#,
        item_id,
        enum_to_text(&s.design),
        s.aperture_mm,
        s.focal_length_mm,
        f64_to_decimal(s.self_weight_kg),
        s.optical_length_mm,
        f64_to_decimal(s.backfocus_mm),
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
                pixel_size_um, sensor_width_px, sensor_height_px,
                self_weight_g, full_well_capacity_e, read_noise_e,
                mount_thread, backfocus_mm
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
        item_id,
        enum_to_text(&s.sensor_type),
        enum_to_text(&s.color_type),
        s.cooled,
        s.sensor_model.as_deref(),
        f64_to_decimal(s.pixel_size_um),
        s.sensor_width_px,
        s.sensor_height_px,
        s.self_weight_g,
        s.full_well_capacity_e,
        f64_to_decimal(s.read_noise_e),
        s.mount_thread.as_deref(),
        f64_to_decimal(s.backfocus_mm),
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
        r#"insert into filter_specs
            (item_id, filter_type, bandwidth_nm, size, mounted,
             mounted_diameter_mm, thickness_mm, peak_transmission_pct)
            values ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        item_id,
        enum_to_text(&s.filter_type),
        f64_to_decimal(s.bandwidth_nm),
        enum_to_text(&s.size),
        s.mounted,
        f64_to_decimal(s.mounted_diameter_mm),
        f64_to_decimal(s.thickness_mm),
        f64_to_decimal(s.peak_transmission_pct),
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
        r#"insert into mount_specs
            (item_id, mount_type, payload_kg, goto,
             self_weight_kg, periodic_error_arcsec, tripod_included, control_protocol)
            values ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        item_id,
        enum_to_text(&s.mount_type),
        f64_to_decimal(s.payload_kg),
        s.goto,
        f64_to_decimal(s.self_weight_kg),
        f64_to_decimal(s.periodic_error_arcsec),
        s.tripod_included,
        s.control_protocol.as_deref(),
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
        r#"insert into focal_modifier_specs
            (item_id, modifier_type, factor,
             self_weight_g, backfocus_mm, image_circle_mm)
            values ($1, $2, $3, $4, $5, $6)"#,
        item_id,
        enum_to_text(&s.modifier_type),
        f64_to_decimal(s.factor),
        s.self_weight_g,
        f64_to_decimal(s.backfocus_mm),
        f64_to_decimal(s.image_circle_mm),
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_guiding(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    s: &GuidingSpecs,
) -> Result<(), AppError> {
    // setup_kind is NOT NULL on the DB row; reject early with a clear
    // 422 instead of letting Postgres surface a generic constraint error.
    let setup_kind = enum_to_text(&s.setup_kind)
        .ok_or_else(|| AppError::Validation("guiding specs require setup_kind".into()))?;
    sqlx::query!(
        r#"insert into guiding_specs
            (item_id, setup_kind, guide_focal_mm, guide_aperture_mm, guide_camera)
            values ($1, $2, $3, $4, $5)"#,
        item_id,
        setup_kind,
        s.guide_focal_mm,
        s.guide_aperture_mm,
        s.guide_camera.as_deref(),
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_types::{EquipmentSpecsPayload, FilterSpecs, GuidingSpecs, TelescopeSpecs};

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
    fn accepts_guiding_kind() {
        let payload = EquipmentSpecsPayload::Guiding(GuidingSpecs::default());
        ensure_matches_kind("guiding", &payload).unwrap();
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
