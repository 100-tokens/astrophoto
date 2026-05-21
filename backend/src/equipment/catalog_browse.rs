//! GET /api/equipment/catalog?kind=&brand=&q=&min_aperture=&max_aperture=&sort=&page=&limit=
//!
//! Backs the `/equip/[kind]` browse page. Returns a paginated list of
//! catalog items (joined to the per-kind spec table) alongside a
//! `facets` block that powers the sidebar filter pane: one bucket
//! count per brand, plus per-kind enum facets (designs for telescopes,
//! sensor_type for cameras, etc).
//!
//! Design notes:
//!   - The endpoint is public — same posture as `/api/equipment/items/:id`.
//!   - Pagination is offset-based. The catalog is small (~100s of items
//!     per kind in the foreseeable future) so cursor pagination is
//!     overkill and would complicate the brand-facet UI ("how many
//!     pages of Sky-Watcher items are there?").
//!   - Facets reflect the catalog BEFORE the brand filter is applied —
//!     the same UX convention as Amazon/Algolia: clicking "Sky-Watcher"
//!     narrows the items but the other brand buckets remain so the
//!     user can pivot. Per-kind enum facets behave the same way.
//!   - The `sort=aperture_desc` option only makes sense for `kind=telescope`.
//!     For other kinds it silently falls back to `usage_count desc`
//!     (the default) so the URL stays bookmarkable when the user
//!     navigates between kinds.
//!   - The brand filter accepts a comma-separated list (`brand=ZWO,Sky-Watcher`)
//!     because that's what the frontend builds from checkbox state.

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::types::BigDecimal;
use uuid::Uuid;

use crate::api_types::{
    CameraColorType, CameraSensorType, CameraSpecs, EquipmentCatalogResponse, EquipmentFacetBucket,
    EquipmentFacets, EquipmentItemDetail, EquipmentSpecsPayload, FilterSize, FilterSpecs,
    FilterType, FocalModifierSpecs, FocalModifierType, GuidingSetupKind, GuidingSpecs, MountSpecs,
    MountType, TelescopeDesign, TelescopeSpecs,
};
use crate::equipment::VALID_KINDS;
use crate::error::AppError;
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 96;

#[derive(Deserialize)]
pub struct Q {
    pub kind: String,
    #[serde(default)]
    pub q: Option<String>,
    /// Comma-separated list of brand display strings (e.g. "Sky-Watcher,ZWO").
    /// Trimmed of empty tokens. Empty/absent = no brand filter.
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub min_aperture: Option<i32>,
    #[serde(default)]
    pub max_aperture: Option<i32>,
    /// One of: most_used (default), brand_asc, aperture_desc, recent.
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Decode the comma-separated `brand=` URL token into the on-disk
/// brand strings. The DB stores unknown brands as `''` (empty string);
/// the facet response surfaces those as the human-readable label
/// `"Unknown"` so the sidebar renders a clickable checkbox. We have
/// to invert that mapping on the way back in so clicking "Unknown"
/// actually filters the items query.
fn parse_brands(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| {
                if t == "Unknown" {
                    String::new()
                } else {
                    t.to_string()
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

fn decimal_to_f64(d: Option<BigDecimal>) -> Option<f64> {
    d.and_then(|x| x.to_string().parse::<f64>().ok())
}

fn parse_enum<T>(s: Option<String>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    s.and_then(|val| serde_json::from_value(serde_json::Value::String(val)).ok())
}

/// Row shape shared by every per-kind item query. The per-kind handler
/// is responsible for hydrating the typed `EquipmentSpecsPayload`
/// variant separately.
struct ItemHeaderRow {
    id: Uuid,
    kind: String,
    canonical_name: String,
    display_name: String,
    usage_count: i32,
    status: String,
    submitted_by: Option<Uuid>,
    approved_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    brand: String,
    model: String,
    variant: Option<String>,
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
    let kind = qs.kind.clone();
    let brands = parse_brands(qs.brand.as_deref());
    let q_pattern = qs.q.as_deref().and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(format!("%{t}%"))
        }
    });
    let limit = qs.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let page = qs.page.unwrap_or(0).max(0);
    let offset = page * limit;
    let sort = qs.sort.as_deref().unwrap_or("most_used");

    // Aperture range filters only meaningful for telescopes; ignored
    // for other kinds (the SQL filter only joins telescope_specs in
    // that branch).
    let (min_ap, max_ap) = if kind == "telescope" {
        (qs.min_aperture, qs.max_aperture)
    } else {
        (None, None)
    };

    // ── Items query: per-kind dispatch. Each branch joins the
    //    correct spec table so we can sort by aperture and emit the
    //    specs payload in the same round trip. The brand-list filter
    //    uses `= ANY($n)` against a text[] so the same prepared
    //    statement handles 0..N brands.
    let brands_filter: Option<&[String]> = if brands.is_empty() {
        None
    } else {
        Some(&brands)
    };

    // `total` reflects the currently active filters (including brand
    // and aperture range) — the frontend reads it to render the page
    // count below the grid. Brand-facet counts are computed separately
    // by `load_facets` against the brand-unfiltered set so the
    // sidebar checkboxes stay clickable after a brand is selected.
    let items = load_items_for_kind(
        &state.pool,
        &kind,
        q_pattern.as_deref(),
        brands_filter,
        min_ap,
        max_ap,
        sort,
        limit,
        offset,
    )
    .await?;

    let total = count_items_for_kind(
        &state.pool,
        &kind,
        q_pattern.as_deref(),
        brands_filter,
        min_ap,
        max_ap,
    )
    .await?;

    let facets = load_facets(&state.pool, &kind, q_pattern.as_deref()).await?;

    Ok(Json(EquipmentCatalogResponse {
        items,
        facets,
        total,
        limit,
        offset,
    }))
}

#[allow(clippy::too_many_arguments)]
async fn load_items_for_kind(
    pool: &sqlx::PgPool,
    kind: &str,
    q_pattern: Option<&str>,
    brands_filter: Option<&[String]>,
    min_ap: Option<i32>,
    max_ap: Option<i32>,
    sort: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<EquipmentItemDetail>, AppError> {
    // Brand filter as an Option<Vec<String>> is the simplest portable
    // shape: when None the SQL `($n::text[] is null or brand = any($n))`
    // short-circuits to true, when Some it constrains.
    let brand_vec: Option<Vec<String>> = brands_filter.map(|s| s.to_vec());

    // Sort dispatch lives in the per-kind branches below — we use a
    // bound-parameter CASE in ORDER BY so the same prepared statement
    // handles every `sort` value without dynamic SQL. `aperture_desc`
    // is only available in the telescope branch (the only kind that
    // joins `telescope_specs`); other kinds silently collapse to the
    // default (most_used) when given `aperture_desc`.

    // We dispatch one query per kind so sqlx can compile-check the
    // per-kind spec columns. The selected columns must stay identical
    // between branches (they hydrate the same `ItemHeaderRow`-shaped
    // intermediate). Per-kind extras come back as separate rows fetched
    // by `load_*_specs` below — cheaper than dynamic SQL.
    match kind {
        "telescope" => {
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       ts.design, ts.aperture_mm, ts.focal_length_mm, ts.focal_ratio_f,
                       ts.self_weight_kg, ts.optical_length_mm, ts.backfocus_mm
                  from equipment_items ei
                  left join telescope_specs ts on ts.item_id = ei.id
                 where ei.kind = 'telescope'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                   and ($3::int4 is null or ts.aperture_mm >= $3)
                   and ($4::int4 is null or ts.aperture_mm <= $4)
                 order by
                    case when $5::text = 'aperture_desc' then ts.aperture_mm end desc nulls last,
                    case when $5::text = 'brand_asc'     then ei.brand end asc,
                    case when $5::text = 'brand_asc'     then ei.model end asc,
                    case when $5::text = 'recent'        then ei.created_at end desc,
                    case when $5::text = 'most_used'     then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $6 offset $7
                "#,
                q_pattern,
                brand_vec.as_deref(),
                min_ap,
                max_ap,
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    let specs = r.aperture_mm.is_some()
                        || r.design.is_some()
                        || r.focal_length_mm.is_some()
                        || r.self_weight_kg.is_some();
                    let payload = if specs {
                        Some(EquipmentSpecsPayload::Telescope(TelescopeSpecs {
                            design: parse_enum::<TelescopeDesign>(r.design),
                            aperture_mm: r.aperture_mm,
                            focal_length_mm: r.focal_length_mm,
                            focal_ratio_f: decimal_to_f64(r.focal_ratio_f),
                            self_weight_kg: decimal_to_f64(r.self_weight_kg),
                            optical_length_mm: r.optical_length_mm,
                            backfocus_mm: decimal_to_f64(r.backfocus_mm),
                        }))
                    } else {
                        None
                    };
                    header_into_detail(header, payload)
                })
                .collect())
        }
        "camera" => {
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       cs.sensor_type, cs.color_type, cs.cooled, cs.sensor_model,
                       cs.pixel_size_um, cs.sensor_width_px, cs.sensor_height_px,
                       cs.self_weight_g, cs.full_well_capacity_e, cs.read_noise_e,
                       cs.mount_thread, cs.backfocus_mm
                  from equipment_items ei
                  left join camera_specs cs on cs.item_id = ei.id
                 where ei.kind = 'camera'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                 order by
                    case when $3::text = 'brand_asc' then ei.brand end asc,
                    case when $3::text = 'brand_asc' then ei.model end asc,
                    case when $3::text = 'recent'    then ei.created_at end desc,
                    case when $3::text = 'most_used' then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $4 offset $5
                "#,
                q_pattern,
                brand_vec.as_deref(),
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    let specs = r.sensor_type.is_some()
                        || r.color_type.is_some()
                        || r.cooled.is_some()
                        || r.sensor_model.is_some();
                    let payload = if specs {
                        Some(EquipmentSpecsPayload::Camera(CameraSpecs {
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
                        }))
                    } else {
                        None
                    };
                    header_into_detail(header, payload)
                })
                .collect())
        }
        "mount" => {
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       ms.mount_type, ms.payload_kg, ms.goto,
                       ms.self_weight_kg, ms.periodic_error_arcsec,
                       ms.tripod_included, ms.control_protocol
                  from equipment_items ei
                  left join mount_specs ms on ms.item_id = ei.id
                 where ei.kind = 'mount'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                 order by
                    case when $3::text = 'brand_asc' then ei.brand end asc,
                    case when $3::text = 'brand_asc' then ei.model end asc,
                    case when $3::text = 'recent'    then ei.created_at end desc,
                    case when $3::text = 'most_used' then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $4 offset $5
                "#,
                q_pattern,
                brand_vec.as_deref(),
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    let specs =
                        r.mount_type.is_some() || r.payload_kg.is_some() || r.goto.is_some();
                    let payload = if specs {
                        Some(EquipmentSpecsPayload::Mount(MountSpecs {
                            mount_type: parse_enum::<MountType>(r.mount_type),
                            payload_kg: decimal_to_f64(r.payload_kg),
                            goto: r.goto,
                            self_weight_kg: decimal_to_f64(r.self_weight_kg),
                            periodic_error_arcsec: decimal_to_f64(r.periodic_error_arcsec),
                            tripod_included: r.tripod_included,
                            control_protocol: r.control_protocol,
                        }))
                    } else {
                        None
                    };
                    header_into_detail(header, payload)
                })
                .collect())
        }
        "filter" => {
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       fs.filter_type, fs.bandwidth_nm, fs.size, fs.mounted,
                       fs.mounted_diameter_mm, fs.thickness_mm, fs.peak_transmission_pct
                  from equipment_items ei
                  left join filter_specs fs on fs.item_id = ei.id
                 where ei.kind = 'filter'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                 order by
                    case when $3::text = 'brand_asc' then ei.brand end asc,
                    case when $3::text = 'brand_asc' then ei.model end asc,
                    case when $3::text = 'recent'    then ei.created_at end desc,
                    case when $3::text = 'most_used' then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $4 offset $5
                "#,
                q_pattern,
                brand_vec.as_deref(),
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    let specs = r.filter_type.is_some()
                        || r.bandwidth_nm.is_some()
                        || r.size.is_some()
                        || r.mounted.is_some();
                    let payload = if specs {
                        Some(EquipmentSpecsPayload::Filter(FilterSpecs {
                            filter_type: parse_enum::<FilterType>(r.filter_type),
                            bandwidth_nm: decimal_to_f64(r.bandwidth_nm),
                            size: parse_enum::<FilterSize>(r.size),
                            mounted: r.mounted,
                            mounted_diameter_mm: decimal_to_f64(r.mounted_diameter_mm),
                            thickness_mm: decimal_to_f64(r.thickness_mm),
                            peak_transmission_pct: decimal_to_f64(r.peak_transmission_pct),
                        }))
                    } else {
                        None
                    };
                    header_into_detail(header, payload)
                })
                .collect())
        }
        "focal_modifier" => {
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       fm.modifier_type, fm.factor,
                       fm.self_weight_g, fm.backfocus_mm, fm.image_circle_mm
                  from equipment_items ei
                  left join focal_modifier_specs fm on fm.item_id = ei.id
                 where ei.kind = 'focal_modifier'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                 order by
                    case when $3::text = 'brand_asc' then ei.brand end asc,
                    case when $3::text = 'brand_asc' then ei.model end asc,
                    case when $3::text = 'recent'    then ei.created_at end desc,
                    case when $3::text = 'most_used' then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $4 offset $5
                "#,
                q_pattern,
                brand_vec.as_deref(),
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    let specs = r.modifier_type.is_some() || r.factor.is_some();
                    let payload = if specs {
                        Some(EquipmentSpecsPayload::FocalModifier(FocalModifierSpecs {
                            modifier_type: parse_enum::<FocalModifierType>(r.modifier_type),
                            factor: decimal_to_f64(r.factor),
                            self_weight_g: r.self_weight_g,
                            backfocus_mm: decimal_to_f64(r.backfocus_mm),
                            image_circle_mm: decimal_to_f64(r.image_circle_mm),
                        }))
                    } else {
                        None
                    };
                    header_into_detail(header, payload)
                })
                .collect())
        }
        "guiding" => {
            // Force Option<String> on setup_kind: sqlx normally infers
            // it as NOT NULL from the table definition, but the LEFT
            // JOIN makes the column legitimately nullable in row-space.
            let rows = sqlx::query!(
                r#"
                select ei.id, ei.kind, ei.canonical_name, ei.display_name, ei.usage_count,
                       ei.status, ei.submitted_by, ei.approved_at, ei.created_at,
                       ei.brand, ei.model, ei.variant,
                       gs.setup_kind as "setup_kind?",
                       gs.guide_focal_mm, gs.guide_aperture_mm, gs.guide_camera
                  from equipment_items ei
                  left join guiding_specs gs on gs.item_id = ei.id
                 where ei.kind = 'guiding'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                   and ($2::text[] is null or ei.brand = any($2))
                 order by
                    case when $3::text = 'brand_asc' then ei.brand end asc,
                    case when $3::text = 'brand_asc' then ei.model end asc,
                    case when $3::text = 'recent'    then ei.created_at end desc,
                    case when $3::text = 'most_used' then ei.usage_count end desc,
                    ei.usage_count desc,
                    ei.canonical_name asc
                 limit $4 offset $5
                "#,
                q_pattern,
                brand_vec.as_deref(),
                sort,
                limit,
                offset,
            )
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let header = ItemHeaderRow {
                        id: r.id,
                        kind: r.kind,
                        canonical_name: r.canonical_name,
                        display_name: r.display_name,
                        usage_count: r.usage_count,
                        status: r.status,
                        submitted_by: r.submitted_by,
                        approved_at: r.approved_at,
                        created_at: r.created_at,
                        brand: r.brand,
                        model: r.model,
                        variant: r.variant,
                    };
                    // setup_kind is NOT NULL in DB but sqlx surfaces
                    // it as Option<String> here because the LEFT JOIN
                    // makes it nullable from the outer-projection
                    // perspective. None means the item has no
                    // `guiding_specs` row attached.
                    let payload = r.setup_kind.map(|sk| {
                        EquipmentSpecsPayload::Guiding(GuidingSpecs {
                            setup_kind: parse_enum::<GuidingSetupKind>(Some(sk)),
                            guide_focal_mm: r.guide_focal_mm,
                            guide_aperture_mm: r.guide_aperture_mm,
                            guide_camera: r.guide_camera,
                        })
                    });
                    header_into_detail(header, payload)
                })
                .collect())
        }
        _ => unreachable!("kind validated up-front"),
    }
}

fn header_into_detail(
    h: ItemHeaderRow,
    specs: Option<EquipmentSpecsPayload>,
) -> EquipmentItemDetail {
    EquipmentItemDetail {
        id: h.id.to_string(),
        kind: h.kind,
        canonical_name: h.canonical_name,
        display_name: h.display_name,
        usage_count: h.usage_count,
        status: h.status,
        submitted_by: h.submitted_by.map(|u| u.to_string()),
        approved_at: h.approved_at.map(|t| t.to_rfc3339()),
        created_at: h.created_at.to_rfc3339(),
        specs,
        brand: h.brand,
        model: h.model,
        variant: h.variant,
        submitted_by_handle: None,
        setup_count: 0,
    }
}

async fn count_items_for_kind(
    pool: &sqlx::PgPool,
    kind: &str,
    q_pattern: Option<&str>,
    brands_filter: Option<&[String]>,
    min_ap: Option<i32>,
    max_ap: Option<i32>,
) -> Result<i64, AppError> {
    let brand_vec: Option<Vec<String>> = brands_filter.map(|s| s.to_vec());
    // For non-telescope kinds the aperture range filter is a no-op.
    if kind == "telescope" {
        let r = sqlx::query!(
            r#"
            select count(*)::int8 as "n!"
              from equipment_items ei
              left join telescope_specs ts on ts.item_id = ei.id
             where ei.kind = 'telescope'
               and ($1::text is null or
                    ei.canonical_name ilike $1 or
                    ei.display_name ilike $1 or
                    (ei.brand || ' ' || ei.model ||
                        coalesce(' ' || ei.variant, '')) ilike $1)
               and ($2::text[] is null or ei.brand = any($2))
               and ($3::int4 is null or ts.aperture_mm >= $3)
               and ($4::int4 is null or ts.aperture_mm <= $4)
            "#,
            q_pattern,
            brand_vec.as_deref(),
            min_ap,
            max_ap,
        )
        .fetch_one(pool)
        .await?;
        Ok(r.n)
    } else {
        let r = sqlx::query!(
            r#"
            select count(*)::int8 as "n!"
              from equipment_items ei
             where ei.kind = $1
               and ($2::text is null or
                    ei.canonical_name ilike $2 or
                    ei.display_name ilike $2 or
                    (ei.brand || ' ' || ei.model ||
                        coalesce(' ' || ei.variant, '')) ilike $2)
               and ($3::text[] is null or ei.brand = any($3))
            "#,
            kind,
            q_pattern,
            brand_vec.as_deref(),
        )
        .fetch_one(pool)
        .await?;
        Ok(r.n)
    }
}

async fn load_facets(
    pool: &sqlx::PgPool,
    kind: &str,
    q_pattern: Option<&str>,
) -> Result<EquipmentFacets, AppError> {
    // Brand facets are kind-scoped and ignore the brand filter
    // (Amazon-style faceting — clicking a brand keeps other brands
    // visible so the user can pivot). They DO honor the search filter,
    // so if the user types "ASI" they only see the brands with
    // matching items.
    let brand_rows = sqlx::query!(
        r#"
        select ei.brand as "brand!", count(*)::int8 as "n!"
          from equipment_items ei
         where ei.kind = $1
           and ($2::text is null or
                ei.canonical_name ilike $2 or
                ei.display_name ilike $2 or
                (ei.brand || ' ' || ei.model ||
                    coalesce(' ' || ei.variant, '')) ilike $2)
         group by ei.brand
         order by count(*) desc, ei.brand asc
        "#,
        kind,
        q_pattern,
    )
    .fetch_all(pool)
    .await?;
    let brands: Vec<EquipmentFacetBucket> = brand_rows
        .into_iter()
        .map(|r| EquipmentFacetBucket {
            // brand="" (unknown) collapses to "Unknown" in the facet
            // label so the bucket renders as a real choice rather than
            // an empty pill the user can't click on.
            value: if r.brand.is_empty() {
                "Unknown".into()
            } else {
                r.brand
            },
            count: r.n,
        })
        .collect();

    let mut facets = EquipmentFacets {
        brands,
        ..Default::default()
    };

    match kind {
        "telescope" => {
            let rows = sqlx::query!(
                r#"
                select coalesce(ts.design, '_none_') as "v!", count(*)::int8 as "n!"
                  from equipment_items ei
                  join telescope_specs ts on ts.item_id = ei.id
                 where ei.kind = 'telescope'
                   and ($1::text is null or
                        ei.canonical_name ilike $1 or
                        ei.display_name ilike $1 or
                        (ei.brand || ' ' || ei.model ||
                            coalesce(' ' || ei.variant, '')) ilike $1)
                 group by ts.design
                 order by count(*) desc
                "#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.designs = Some(
                rows.into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        "camera" => {
            let sensors = sqlx::query!(
                r#"select coalesce(cs.sensor_type, '_none_') as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join camera_specs cs on cs.item_id = ei.id
                    where ei.kind = 'camera'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by cs.sensor_type
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.sensor_types = Some(
                sensors
                    .into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );

            let colors = sqlx::query!(
                r#"select coalesce(cs.color_type, '_none_') as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join camera_specs cs on cs.item_id = ei.id
                    where ei.kind = 'camera'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by cs.color_type
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.color_types = Some(
                colors
                    .into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );

            let cooled = sqlx::query!(
                r#"select case when cs.cooled is null then '_none_'
                              when cs.cooled then 'yes' else 'no' end as "v!",
                          count(*)::int8 as "n!"
                     from equipment_items ei
                     join camera_specs cs on cs.item_id = ei.id
                    where ei.kind = 'camera'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by 1
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.cooled = Some(
                cooled
                    .into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        "mount" => {
            let rows = sqlx::query!(
                r#"select coalesce(ms.mount_type, '_none_') as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join mount_specs ms on ms.item_id = ei.id
                    where ei.kind = 'mount'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by ms.mount_type
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.mount_types = Some(
                rows.into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        "filter" => {
            let rows = sqlx::query!(
                r#"select coalesce(fs.filter_type, '_none_') as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join filter_specs fs on fs.item_id = ei.id
                    where ei.kind = 'filter'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by fs.filter_type
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.filter_types = Some(
                rows.into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        "focal_modifier" => {
            let rows = sqlx::query!(
                r#"select coalesce(fm.modifier_type, '_none_') as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join focal_modifier_specs fm on fm.item_id = ei.id
                    where ei.kind = 'focal_modifier'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by fm.modifier_type
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.modifier_types = Some(
                rows.into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        "guiding" => {
            let rows = sqlx::query!(
                r#"select gs.setup_kind as "v!", count(*)::int8 as "n!"
                     from equipment_items ei
                     join guiding_specs gs on gs.item_id = ei.id
                    where ei.kind = 'guiding'
                      and ($1::text is null or
                           ei.canonical_name ilike $1 or
                           ei.display_name ilike $1 or
                           (ei.brand || ' ' || ei.model ||
                                coalesce(' ' || ei.variant, '')) ilike $1)
                    group by gs.setup_kind
                    order by count(*) desc"#,
                q_pattern,
            )
            .fetch_all(pool)
            .await?;
            facets.setup_kinds = Some(
                rows.into_iter()
                    .map(|r| (r.v, r.n))
                    .filter(filter_none)
                    .map(into_bucket)
                    .collect(),
            );
        }
        _ => unreachable!(),
    }
    Ok(facets)
}

/// Drop the `_none_` sentinel rows we emit from the per-facet queries
/// (rows with `<col> IS NULL` collapse into one synthetic bucket so we
/// can still produce a single GROUP BY result — the bucket itself
/// isn't a real facet value and would render as an empty checkbox).
fn filter_none(t: &(String, i64)) -> bool {
    t.0 != "_none_"
}

fn into_bucket(t: (String, i64)) -> EquipmentFacetBucket {
    EquipmentFacetBucket {
        value: t.0,
        count: t.1,
    }
}
