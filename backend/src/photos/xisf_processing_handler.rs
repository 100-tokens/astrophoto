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

use crate::auth::middleware::OptionalUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::xisf_processing::ProcessingReport;

/// Generic (non-chart) tables larger than this are truncated — the
/// only tables that legitimately grow big are per-subframe file lists,
/// which the path scrub below empties anyway; this is the backstop.
const MAX_TABLE_ROWS: usize = 512;

/// Table cells get the same ceiling as step parameters
/// ([`crate::photos::xisf_processing`]'s `MAX_PARAM_VALUE_LEN`); the
/// parser deliberately stores cells verbatim so the boundary decides.
const MAX_CELL_LEN: usize = 512;

pub async fn handler(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<ProcessingReport>>, AppError> {
    // Drafts are owner-only everywhere else (`GET /api/photos/:id`
    // gates via is_visible_to) — their processing history must not be
    // fetchable by UUID either. Invisible → same null body as "no
    // report", so draft existence isn't probeable.
    let viewer = user.0.as_ref().map(|u| u.id);
    if !crate::photos::queries::is_visible_to(&state.pool, id, viewer).await? {
        return Ok(Json(None));
    }

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
/// Table rows get the same treatment: WBPP/ImageIntegration write one
/// row per subframe with the photographer's absolute local paths
/// (`/Volumes/…/Light_….xisf`), which used to flow through untouched
/// (~226 KB per photo in every anonymous SSR payload). Numeric chart
/// tables (curve/histogram/channels) never match the heuristic and
/// survive intact.
fn sanitize(report: &mut ProcessingReport) {
    for step in &mut report.pipeline {
        step.params.retain(|p| !looks_like_path(&p.value));
        for table in &mut step.tables {
            table
                .rows
                .retain(|row| !row.iter().any(|c| looks_like_path(c)));
            table.rows.truncate(MAX_TABLE_ROWS);
            for row in &mut table.rows {
                for cell in row.iter_mut() {
                    truncate_on_char_boundary(cell, MAX_CELL_LEN);
                }
            }
        }
    }
    // Privacy: precise site coordinates are parsed and stored, but never
    // exposed on the public endpoint (the XISF header embeds exact GPS).
    if let Some(obs) = report.observation.as_mut() {
        obs.site_latitude = None;
        obs.site_longitude = None;
        obs.site_elevation_m = None;
    }
}

/// `String::truncate` panics mid-code-point; back off to a boundary.
fn truncate_on_char_boundary(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
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

    fn step_with_tables(tables: Vec<crate::photos::xisf_processing::ProcessTable>) -> ProcessStep {
        ProcessStep {
            position: 0,
            class_name: "ImageIntegration".into(),
            label: "Image Integration".into(),
            category: "Stacking".into(),
            summary: None,
            version: None,
            enabled: true,
            started_at: None,
            duration_s: None,
            params: vec![],
            tables,
        }
    }

    fn report_with(steps: Vec<ProcessStep>) -> ProcessingReport {
        ProcessingReport {
            creator_app: None,
            creator_module: None,
            creator_os: None,
            created_at: None,
            display_stretch: None,
            white_balance: None,
            observation: None,
            total_duration_s: None,
            pipeline: steps,
        }
    }

    #[test]
    fn sanitize_drops_path_rows_from_tables_keeps_numeric() {
        use crate::photos::xisf_processing::ProcessTable;
        let file_list = ProcessTable {
            id: "images".into(),
            kind: "generic".into(),
            columns: vec!["enabled".into(), "path".into()],
            rows: vec![
                vec![
                    "true".into(),
                    "/Volumes/Pascal4Tb/astrophotos/NGC5982/Light_300s_0001.xisf".into(),
                ],
                vec![
                    "true".into(),
                    "/Volumes/Pascal4Tb/astrophotos/NGC5982/Light_300s_0002.xisf".into(),
                ],
            ],
        };
        let weights = ProcessTable {
            id: "weights".into(),
            kind: "generic".into(),
            columns: vec!["weightRK".into(), "weightG".into()],
            rows: vec![
                vec!["0.39693".into(), "0.00000".into()],
                vec!["0.44206".into(), "0.00000".into()],
            ],
        };
        let curve = ProcessTable {
            id: "K".into(),
            kind: "curve".into(),
            columns: vec!["x".into(), "y".into()],
            rows: vec![
                vec!["0.0".into(), "0.0".into()],
                vec!["1.0".into(), "1.0".into()],
            ],
        };
        let mut report = report_with(vec![step_with_tables(vec![file_list, weights, curve])]);
        sanitize(&mut report);
        let tables = &report.pipeline[0].tables;
        assert!(tables[0].rows.is_empty(), "subframe path rows dropped");
        assert_eq!(tables[1].rows.len(), 2, "numeric weight rows kept");
        assert_eq!(tables[2].rows.len(), 2, "curve chart rows kept");
        assert_eq!(tables[2].rows[1], vec!["1.0", "1.0"]);
    }

    #[test]
    fn sanitize_caps_table_rows_and_cell_length() {
        use crate::photos::xisf_processing::ProcessTable;
        let big = ProcessTable {
            id: "t".into(),
            kind: "generic".into(),
            columns: vec!["v".into()],
            rows: (0..MAX_TABLE_ROWS + 100)
                .map(|i| vec![format!("{i}")])
                .collect(),
        };
        let long_cell = ProcessTable {
            id: "l".into(),
            kind: "generic".into(),
            columns: vec!["v".into()],
            // Multi-byte char straddling the cap must not panic.
            rows: vec![vec!["é".repeat(MAX_CELL_LEN)]],
        };
        let mut report = report_with(vec![step_with_tables(vec![big, long_cell])]);
        sanitize(&mut report);
        assert_eq!(report.pipeline[0].tables[0].rows.len(), MAX_TABLE_ROWS);
        let cell = &report.pipeline[0].tables[1].rows[0][0];
        assert!(cell.len() <= MAX_CELL_LEN);
        assert!(cell.chars().all(|c| c == 'é'), "no split code point");
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
