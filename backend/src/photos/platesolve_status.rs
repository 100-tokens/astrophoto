//! `GET /api/photos/:id/platesolve-status` — owner-only read of a
//! photo's plate-solve telemetry, intended for the verify-form
//! polling loop and any future "show astrometry" UI.
//!
//! Mounted unconditionally (unlike `POST /platesolve`, which only
//! mounts when the upstream client is configured): even a deployment
//! with `APP_PLATESOLVE_BASE_URL` unset still wants to surface past
//! results that landed via another instance / a backfill.

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::platesolve::SOLVING_SENTINEL;

/// Shape returned by `/platesolve-status`. The `state` discriminant
/// is what the UI branches on; the other fields are populated only
/// when relevant to the state (telemetry on `Solved`, message on
/// `Failed`).
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "PlatesolveStatus.ts", rename_all = "camelCase")]
pub struct PlatesolveStatus {
    /// One of `idle` / `solving` / `solved` / `failed`. Discriminator
    /// for the UI; the JSON body always includes the same field set
    /// so frontend types stay stable.
    pub state: PlatesolveState,
    /// Human-readable failure reason. Populated only when
    /// `state == Failed`. (We deliberately surface the
    /// `ABORTED_SENTINEL` string as a regular failure so the UI can
    /// render "you can retry to clear" without special-casing it.)
    pub error: Option<String>,
    /// RFC 3339 timestamp of the successful solve. Populated only
    /// when `state == Solved`.
    pub solved_at: Option<String>,
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
    pub pixel_scale_arcsec: Option<f64>,
    pub rotation_deg: Option<f64>,
    pub rms_arcsec: Option<f64>,
    pub matched_count: Option<i32>,
    pub detected_count: Option<i32>,
}

#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "PlatesolveStatus.ts", rename_all = "lowercase")]
pub enum PlatesolveState {
    /// Never attempted — no solve has been run for this photo.
    Idle,
    /// A solve is in flight (sentinel set, no result yet).
    Solving,
    /// A solve completed and the telemetry below is current.
    Solved,
    /// The last attempt failed. `error` carries the reason.
    Failed,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PlatesolveStatus>, AppError> {
    // Runtime query (not `sqlx::query!`) — see the rationale in
    // `crate::photos::platesolve::save_result`. Once migration 0021
    // is applied to a dev DB + `cargo sqlx prepare` runs, this can
    // promote to the compile-time form.
    let row: Option<PlatesolveRow> = sqlx::query_as::<_, PlatesolveRow>(
        r#"select owner_id, ra_deg, dec_deg,
                  platesolve_solved_at, platesolve_error,
                  platesolve_pixel_scale_arcsec, platesolve_rotation_deg,
                  platesolve_rms_arcsec, platesolve_matched_count,
                  platesolve_detected_count
             from photos where id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("photo"));
    };
    if row.owner_id != user.id {
        // Same leak-prevention pattern as `upload_finalize` / the
        // POST /platesolve handler — hide existence under 404.
        return Err(AppError::not_found("photo"));
    }

    let (state_kind, error) = classify(&row);

    Ok(Json(PlatesolveStatus {
        state: state_kind,
        error,
        solved_at: row.platesolve_solved_at.map(|d| d.to_rfc3339()),
        ra_deg: row.ra_deg,
        dec_deg: row.dec_deg,
        pixel_scale_arcsec: row.platesolve_pixel_scale_arcsec.map(f64::from),
        rotation_deg: row.platesolve_rotation_deg.map(f64::from),
        rms_arcsec: row.platesolve_rms_arcsec.map(f64::from),
        matched_count: row.platesolve_matched_count,
        detected_count: row.platesolve_detected_count,
    }))
}

fn classify(row: &PlatesolveRow) -> (PlatesolveState, Option<String>) {
    // Solved takes precedence over a stale error — the success path
    // clears `platesolve_error` so the only way both are set is a
    // backfill bug, in which case `solved_at` is the source of truth.
    if row.platesolve_solved_at.is_some() {
        return (PlatesolveState::Solved, None);
    }
    match row.platesolve_error.as_deref() {
        Some(SOLVING_SENTINEL) => (PlatesolveState::Solving, None),
        Some(msg) => (PlatesolveState::Failed, Some(msg.to_string())),
        None => (PlatesolveState::Idle, None),
    }
}

#[derive(sqlx::FromRow)]
struct PlatesolveRow {
    owner_id: Uuid,
    ra_deg: Option<f64>,
    dec_deg: Option<f64>,
    platesolve_solved_at: Option<DateTime<Utc>>,
    platesolve_error: Option<String>,
    platesolve_pixel_scale_arcsec: Option<f32>,
    platesolve_rotation_deg: Option<f32>,
    platesolve_rms_arcsec: Option<f32>,
    platesolve_matched_count: Option<i32>,
    platesolve_detected_count: Option<i32>,
}
