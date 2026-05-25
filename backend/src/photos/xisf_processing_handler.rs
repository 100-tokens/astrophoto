//! `GET /api/photos/:id/processing` — public, sanitized view of the
//! XISF processing report parsed at upload time into
//! `photos.processing_json`.
//!
//! No auth: the photo permalink page is public. Sanitization happens
//! here (at the boundary) rather than at write time, so the stored
//! report stays complete and the policy can evolve without re-parsing.
//! Returns a `null` body (200) when the photo has no report — so the
//! frontend simply renders nothing.

use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;
use crate::http::AppState;
use crate::photos::xisf_processing::ProcessingReport;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<ProcessingReport>>, AppError> {
    let row: Option<(Option<Value>,)> =
        sqlx::query_as::<_, (Option<Value>,)>("SELECT processing_json FROM photos WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(AppError::from)?;

    // Photo missing OR no report → null body. We deliberately don't 404
    // on a missing photo: this is a public, best-effort enrichment
    // endpoint and the page already 404s via the permalink lookup.
    let Some((Some(json),)) = row else {
        return Ok(Json(None));
    };

    let mut report: ProcessingReport = serde_json::from_value(json)
        .map_err(|e| AppError::Internal(format!("decode processing_json: {e}")))?;
    sanitize(&mut report);
    Ok(Json(Some(report)))
}

/// Drop params whose value looks like a filesystem path or a bare hash —
/// these leak machine details and carry no value to a public reader.
fn sanitize(report: &mut ProcessingReport) {
    for step in &mut report.pipeline {
        step.params.retain(|p| !looks_like_path(&p.value));
    }
    // Privacy: precise site coordinates are parsed and stored, but never
    // exposed on the public endpoint (the XISF header embeds exact GPS).
    if let Some(obs) = report.observation.as_mut() {
        obs.site_latitude = None;
        obs.site_longitude = None;
        obs.site_elevation_m = None;
    }
}

fn looks_like_path(v: &str) -> bool {
    let v = v.trim();
    if v.is_empty() {
        return false;
    }
    // Unix-absolute, home-relative, or PixInsight env-var paths.
    if v.starts_with('/') || v.starts_with("~/") || v.starts_with('$') {
        return true;
    }
    // Windows drive path: `C:\...`.
    if v.len() >= 3 && v.as_bytes()[1] == b':' && v.contains('\\') {
        return true;
    }
    // Relative path with a file extension: `foo/bar.ext`.
    if v.contains('/') && v.contains('.') {
        return true;
    }
    // Bare 32/40-char hex digest (md5 / sha1).
    if (v.len() == 32 || v.len() == 40) && v.bytes().all(|b| b.is_ascii_hexdigit()) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photos::xisf_processing::{KeyValue, ProcessStep, ProcessingReport};

    fn kv(k: &str, v: &str) -> KeyValue {
        KeyValue {
            key: k.into(),
            value: v.into(),
            truncated: false,
        }
    }

    #[test]
    fn strips_paths_keeps_identifiers() {
        // dropped: machine paths and bare hashes
        assert!(looks_like_path("$PXI_SRCDIR/scripts/AdP/ImageSolver.js"));
        assert!(looks_like_path("/Users/me/data/light.xisf"));
        assert!(looks_like_path(r"C:\Users\me\light.xisf"));
        assert!(looks_like_path("~/astro/masters/m20.xisf"));
        assert!(looks_like_path("fe992f408d2f2de770c7ce87451c548b")); // md5
        // kept: meaningful identifiers and values
        assert!(!looks_like_path("GaiaDR3SP"));
        assert!(!looks_like_path("BlurXTerminator.4.mlpackage"));
        assert!(!looks_like_path("ImageSolver 6.3.1"));
        assert!(!looks_like_path("0.05"));
        assert!(!looks_like_path("K0V Star"));
        assert!(!looks_like_path("RGB"));
    }

    #[test]
    fn sanitize_drops_path_params_only() {
        let mut report = ProcessingReport {
            creator_app: None,
            creator_module: None,
            creator_os: None,
            created_at: None,
            display_stretch: None,
            white_balance: None,
            observation: None,
            total_duration_s: None,
            pipeline: vec![ProcessStep {
                position: 0,
                class_name: "Script".into(),
                label: "Script".into(),
                category: "Bookkeeping".into(),
                summary: None,
                version: None,
                enabled: true,
                started_at: None,
                duration_s: None,
                params: vec![
                    kv("filePath", "$PXI_SRCDIR/x.js"),
                    kv("information", "ImageSolver 6.3.1"),
                ],
                tables: vec![],
            }],
        };
        sanitize(&mut report);
        let keys: Vec<&str> = report.pipeline[0]
            .params
            .iter()
            .map(|p| p.key.as_str())
            .collect();
        assert_eq!(keys, vec!["information"]);
    }

    #[test]
    fn sanitize_strips_site_coordinates() {
        use crate::photos::xisf_processing::ObservationSummary;
        let mut report = ProcessingReport {
            creator_app: None,
            creator_module: None,
            creator_os: None,
            created_at: None,
            display_stretch: None,
            white_balance: None,
            observation: Some(ObservationSummary {
                filter: Some("L".into()),
                telescope: Some("C8 EDGE HD".into()),
                site_latitude: Some(38.165),
                site_longitude: Some(-2.327),
                site_elevation_m: Some(0.0),
                ..Default::default()
            }),
            total_duration_s: None,
            pipeline: vec![],
        };
        sanitize(&mut report);
        let obs = report.observation.unwrap();
        assert!(obs.site_latitude.is_none(), "lat stripped");
        assert!(obs.site_longitude.is_none(), "lon stripped");
        assert!(obs.site_elevation_m.is_none(), "elevation stripped");
        // non-site fields are preserved
        assert_eq!(obs.filter.as_deref(), Some("L"));
        assert_eq!(obs.telescope.as_deref(), Some("C8 EDGE HD"));
    }
}
