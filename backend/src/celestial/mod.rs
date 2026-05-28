//! Celestial-object identification (D5/D6 from celestial-objects spec).
//! Cone search + write to photo_targets at plate-solve time.
//!
//! Submodules are added incrementally — see plan
//! `docs/superpowers/plans/2026-05-28-celestial-identify-overlay-plan.md`.

pub mod confidence;
pub mod handler;
pub mod identify;
pub mod queries;

pub use identify::{identify, IdentifyOutcome};

/// DB row shape for `GET /celestial-objects`. Owned by this module so
/// the public API type (`crate::api_types::CelestialObject`) stays a
/// pure DTO with no sqlx coupling.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct CelestialObjectRow {
    pub slug: String,
    pub canonical_name: String,
    pub kind: String,
    pub object_type: Option<String>,
    pub magnitude_v: Option<f32>,
    pub right_ascension: f64,
    pub declination: f64,
    pub major_axis_arcmin: Option<f32>,
    pub minor_axis_arcmin: Option<f32>,
    pub position_angle_deg: Option<f32>,
    pub confidence: f32,
}

impl From<CelestialObjectRow> for crate::api_types::CelestialObject {
    fn from(r: CelestialObjectRow) -> Self {
        Self {
            slug: r.slug,
            canonical_name: r.canonical_name,
            kind: r.kind,
            object_type: r.object_type,
            magnitude_v: r.magnitude_v,
            right_ascension: r.right_ascension,
            declination: r.declination,
            major_axis_arcmin: r.major_axis_arcmin,
            minor_axis_arcmin: r.minor_axis_arcmin,
            position_angle_deg: r.position_angle_deg,
            confidence: r.confidence,
        }
    }
}
