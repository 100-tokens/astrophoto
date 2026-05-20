//! GET /api/equipment/autocomplete?kind=<kind>&q=<query>
//!
//! Returns up to 10 equipment_items rows for the given kind, matching ILIKE
//! on canonical_name OR display_name, ordered by usage_count DESC.
//! Public endpoint — no auth required.
//! Empty `q` returns an empty array immediately without touching the DB.
//! Invalid `kind` returns 422 Validation.
//!
//! Catalog v2 (Phase 2 — saisie forcée): each row carries the structured
//! brand/model from the header plus a short `specs_summary` formatted from
//! the joined per-kind specs row (e.g. "100/550 f/5.5" for a telescope,
//! "OIII 6 nm" for a filter). The frontend autocomplete uses these to
//! render `<strong>brand</strong> · model` and a small spec line under
//! each suggestion so users can pick the right entry without opening it.

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;

use crate::equipment::VALID_KINDS;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub kind: String,
    pub q: String,
}

#[derive(Serialize)]
pub struct Item {
    pub id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub usage_count: i32,
    /// Catalog v2 (migration 0022): structured header. `brand=""` denotes
    /// an unknown brand (freetext-created row) — the frontend renders it
    /// as a plain `display_name` in that case.
    pub brand: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Short per-kind spec summary string, computed server-side from the
    /// joined `<kind>_specs` row. `None` when there's no spec row or no
    /// useful fields are populated. Examples:
    ///   - telescope: "100/550 f/5.5"
    ///   - camera:    "IMX571 · 3.76 µm · cooled"
    ///   - mount:     "Eq. German · 20 kg payload · GoTo"
    ///   - filter:    "OIII · 6 nm"
    ///   - focal_mod: "Reducer ×0.79"
    ///   - guiding:   "OAG · 60 mm"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs_summary: Option<String>,
    /// Only populated when `kind = 'filter'` and the item has a
    /// `filter_specs` row — lets `FilterChipInput` render the popup
    /// chip with its real badge + bandwidth instead of "?".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_nm: Option<f64>,
}

#[derive(Serialize)]
pub struct R {
    pub items: Vec<Item>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(qs): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    if !VALID_KINDS.contains(&qs.kind.as_str()) {
        return Err(AppError::Validation(
            "kind must be telescope|camera|mount|filter|focal_modifier|guiding".into(),
        ));
    }
    let q = qs.q.trim();
    if q.is_empty() {
        return Ok(Json(R { items: vec![] }));
    }
    let pattern = format!("%{q}%");
    // Single query, six LEFT JOINs — one per spec sub-table. Each join is
    // qualified on `ei.kind = '<x>'` so a row only ever pulls from its own
    // sub-table; the others contribute NULLs. Cheap because the spec
    // tables are PK-joined on item_id and the kind filter is selective.
    let rows = sqlx::query!(
        r#"
        select ei.id, ei.canonical_name, ei.display_name, ei.usage_count,
               ei.brand, ei.model, ei.variant,
               -- filter
               fs.filter_type     as fs_filter_type,
               fs.bandwidth_nm    as fs_bandwidth_nm,
               -- telescope
               ts.aperture_mm     as ts_aperture_mm,
               ts.focal_length_mm as ts_focal_length_mm,
               ts.focal_ratio_f   as ts_focal_ratio_f,
               -- camera
               cs.sensor_model    as cs_sensor_model,
               cs.pixel_size_um   as cs_pixel_size_um,
               cs.cooled          as cs_cooled,
               -- mount
               ms.mount_type      as ms_mount_type,
               ms.payload_kg      as ms_payload_kg,
               ms.goto            as ms_goto,
               -- focal modifier
               fms.modifier_type  as fms_modifier_type,
               fms.factor         as fms_factor,
               -- guiding
               gs.setup_kind      as gs_setup_kind,
               gs.guide_focal_mm  as gs_guide_focal_mm
          from equipment_items ei
          left join filter_specs         fs  on fs.item_id  = ei.id and ei.kind = 'filter'
          left join telescope_specs      ts  on ts.item_id  = ei.id and ei.kind = 'telescope'
          left join camera_specs         cs  on cs.item_id  = ei.id and ei.kind = 'camera'
          left join mount_specs          ms  on ms.item_id  = ei.id and ei.kind = 'mount'
          left join focal_modifier_specs fms on fms.item_id = ei.id and ei.kind = 'focal_modifier'
          left join guiding_specs        gs  on gs.item_id  = ei.id and ei.kind = 'guiding'
         where ei.kind = $1
           and (ei.canonical_name ilike $2 or ei.display_name ilike $2)
         order by ei.usage_count desc
         limit 10
        "#,
        qs.kind,
        pattern
    )
    .fetch_all(&state.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| {
            let bandwidth_nm = decimal_to_f64(r.fs_bandwidth_nm.clone());
            let specs_summary = match qs.kind.as_str() {
                "telescope" => summarize_telescope(
                    r.ts_aperture_mm,
                    r.ts_focal_length_mm,
                    decimal_to_f64(r.ts_focal_ratio_f),
                ),
                "camera" => summarize_camera(
                    r.cs_sensor_model,
                    decimal_to_f64(r.cs_pixel_size_um),
                    r.cs_cooled,
                ),
                "mount" => {
                    summarize_mount(r.ms_mount_type, decimal_to_f64(r.ms_payload_kg), r.ms_goto)
                }
                "filter" => summarize_filter(r.fs_filter_type.clone(), bandwidth_nm),
                "focal_modifier" => {
                    summarize_focal_modifier(r.fms_modifier_type, decimal_to_f64(r.fms_factor))
                }
                "guiding" => summarize_guiding(r.gs_setup_kind, r.gs_guide_focal_mm),
                _ => None,
            };
            Item {
                id: r.id.to_string(),
                canonical_name: r.canonical_name,
                display_name: r.display_name,
                usage_count: r.usage_count,
                brand: r.brand,
                model: r.model,
                variant: r.variant,
                specs_summary,
                filter_type: r.fs_filter_type,
                bandwidth_nm,
            }
        })
        .collect();
    Ok(Json(R { items }))
}

fn decimal_to_f64(d: Option<BigDecimal>) -> Option<f64> {
    d.and_then(|x| x.to_string().parse::<f64>().ok())
}

/// Format a numeric f-ratio as "f/X.X" — trims trailing zero on integer
/// values (so 5.0 → f/5, 5.5 → f/5.5).
fn fmt_f_ratio(f: f64) -> String {
    let rounded = (f * 10.0).round() / 10.0;
    if (rounded - rounded.trunc()).abs() < f64::EPSILON {
        format!("f/{}", rounded.trunc() as i64)
    } else {
        format!("f/{rounded}")
    }
}

/// Telescope spec line: "100/550 f/5.5". Returns None when both aperture
/// and focal length are missing — partial data still produces a useful
/// fragment (e.g. "100mm" if only aperture is known).
fn summarize_telescope(
    aperture_mm: Option<i32>,
    focal_length_mm: Option<i32>,
    focal_ratio_f: Option<f64>,
) -> Option<String> {
    match (aperture_mm, focal_length_mm) {
        (Some(a), Some(f)) => {
            let ratio = focal_ratio_f
                .map(fmt_f_ratio)
                .map(|s| format!(" {s}"))
                .unwrap_or_default();
            Some(format!("{a}/{f}{ratio}"))
        }
        (Some(a), None) => Some(format!("{a} mm")),
        (None, Some(f)) => Some(format!("{f} mm fl")),
        (None, None) => None,
    }
}

/// Camera spec line: "IMX571 · 3.76 µm · cooled". Only includes fields
/// that are populated; skips entirely when nothing is set.
fn summarize_camera(
    sensor_model: Option<String>,
    pixel_size_um: Option<f64>,
    cooled: Option<bool>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = sensor_model.filter(|s| !s.trim().is_empty()) {
        parts.push(s);
    }
    if let Some(p) = pixel_size_um {
        parts.push(format!("{p} µm"));
    }
    if let Some(true) = cooled {
        parts.push("cooled".into());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

/// Mount spec line: "Eq. German · 20 kg · GoTo".
fn summarize_mount(
    mount_type: Option<String>,
    payload_kg: Option<f64>,
    goto: Option<bool>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(t) = mount_type {
        parts.push(mount_type_label(&t).to_string());
    }
    if let Some(p) = payload_kg {
        parts.push(format!("{p} kg"));
    }
    if let Some(true) = goto {
        parts.push("GoTo".into());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

fn mount_type_label(t: &str) -> &'static str {
    match t {
        "equatorial_german" => "Eq. German",
        "equatorial_fork" => "Eq. Fork",
        "alt_az" => "Alt-Az",
        "harmonic_drive" => "Harmonic",
        "strain_wave" => "Strain wave",
        _ => "Other",
    }
}

/// Filter spec line: "OIII · 6 nm". When bandwidth is missing returns
/// just the type label.
fn summarize_filter(filter_type: Option<String>, bandwidth_nm: Option<f64>) -> Option<String> {
    let t = filter_type.map(|t| filter_type_label(&t).to_string());
    match (t, bandwidth_nm) {
        (Some(t), Some(bw)) => Some(format!("{t} · {bw} nm")),
        (Some(t), None) => Some(t),
        (None, Some(bw)) => Some(format!("{bw} nm")),
        (None, None) => None,
    }
}

fn filter_type_label(t: &str) -> &'static str {
    match t {
        "luminance" => "L",
        "red" => "R",
        "green" => "G",
        "blue" => "B",
        "h_alpha" => "Hα",
        "oiii" => "OIII",
        "sii" => "SII",
        "uv_ir_cut" => "UV/IR cut",
        "dual_band" => "Dual",
        "tri_band" => "Tri",
        "quad_band" => "Quad",
        "light_pollution" => "LPS",
        "broadband_color" => "Broadband",
        _ => "Other",
    }
}

/// Focal modifier spec line: "Reducer ×0.79".
fn summarize_focal_modifier(modifier_type: Option<String>, factor: Option<f64>) -> Option<String> {
    let t = modifier_type.map(|t| focal_modifier_label(&t).to_string());
    match (t, factor) {
        (Some(t), Some(f)) => Some(format!("{t} ×{f}")),
        (Some(t), None) => Some(t),
        (None, Some(f)) => Some(format!("×{f}")),
        (None, None) => None,
    }
}

fn focal_modifier_label(t: &str) -> &'static str {
    match t {
        "reducer" => "Reducer",
        "flattener" => "Flattener",
        "reducer_flattener" => "Reducer+Flat",
        "barlow" => "Barlow",
        "extender" => "Extender",
        "corrector" => "Corrector",
        _ => "Modifier",
    }
}

/// Guiding spec line: "OAG · 60 mm".
fn summarize_guiding(setup_kind: Option<String>, guide_focal_mm: Option<i32>) -> Option<String> {
    let t = setup_kind.map(|t| guiding_label(&t).to_string());
    match (t, guide_focal_mm) {
        (Some(t), Some(f)) => Some(format!("{t} · {f} mm")),
        (Some(t), None) => Some(t),
        (None, Some(f)) => Some(format!("{f} mm")),
        (None, None) => None,
    }
}

fn guiding_label(t: &str) -> &'static str {
    match t {
        "oag" => "OAG",
        "guidescope" => "Guidescope",
        "oag_prism" => "OAG prism",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telescope_summary_full() {
        assert_eq!(
            summarize_telescope(Some(100), Some(550), Some(5.5)),
            Some("100/550 f/5.5".to_string())
        );
    }

    #[test]
    fn telescope_summary_integer_ratio() {
        // f/5.0 displays as "f/5" — keep the line short.
        assert_eq!(
            summarize_telescope(Some(80), Some(400), Some(5.0)),
            Some("80/400 f/5".to_string())
        );
    }

    #[test]
    fn telescope_summary_partial() {
        assert_eq!(
            summarize_telescope(Some(100), None, None),
            Some("100 mm".to_string())
        );
        assert_eq!(summarize_telescope(None, None, None), None);
    }

    #[test]
    fn filter_summary() {
        assert_eq!(
            summarize_filter(Some("oiii".into()), Some(6.0)),
            Some("OIII · 6 nm".to_string())
        );
        assert_eq!(
            summarize_filter(Some("luminance".into()), None),
            Some("L".to_string())
        );
        assert_eq!(summarize_filter(None, None), None);
    }

    #[test]
    fn mount_summary() {
        assert_eq!(
            summarize_mount(Some("equatorial_german".into()), Some(20.0), Some(true)),
            Some("Eq. German · 20 kg · GoTo".to_string())
        );
    }

    #[test]
    fn camera_summary_skips_empty() {
        assert_eq!(summarize_camera(Some("   ".into()), None, None), None);
        assert_eq!(
            summarize_camera(Some("IMX571".into()), Some(3.76), Some(true)),
            Some("IMX571 · 3.76 µm · cooled".to_string())
        );
    }
}
