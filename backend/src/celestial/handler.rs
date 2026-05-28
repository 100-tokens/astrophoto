//! HTTP endpoints for celestial objects. See spec §5.

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::api_types::CelestialObject;
use crate::auth::middleware::{CurrentUser, OptionalUser};
use crate::error::AppError;
use crate::http::AppState;

#[derive(serde::Serialize)]
pub struct ListResponse {
    pub objects: Vec<CelestialObject>,
}

/// `GET /api/photos/:id/celestial-objects` — public if the photo is
/// published; owner-only for drafts.
pub async fn list(
    State(state): State<AppState>,
    OptionalUser(viewer): OptionalUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<ListResponse>, AppError> {
    let row = sqlx::query!(
        "select owner_id, published_at from photos where id = $1",
        photo_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    if row.published_at.is_none() {
        let viewer_id = viewer.ok_or(AppError::Unauthorized)?.id;
        if viewer_id != row.owner_id {
            return Err(AppError::Forbidden);
        }
    }

    let rows = sqlx::query_as!(
        crate::celestial::CelestialObjectRow,
        r#"select t.slug, t.canonical_name, t.kind,
                  t.object_type, t.magnitude_v,
                  t.right_ascension as "right_ascension!",
                  t.declination     as "declination!",
                  t.major_axis_arcmin, t.minor_axis_arcmin, t.position_angle_deg,
                  pt.confidence::float4 as "confidence!"
             from photo_targets pt
             join targets t on t.id = pt.target_id
            where pt.photo_id = $1
              and pt.source = 'plate_solve'
            order by pt.confidence desc nulls last,
                     t.magnitude_v asc nulls last"#,
        photo_id,
    )
    .fetch_all(&state.pool)
    .await?;

    let objects: Vec<CelestialObject> = rows.into_iter().map(Into::into).collect();
    Ok(Json(ListResponse { objects }))
}

#[derive(serde::Serialize)]
pub struct RecomputeResponse {
    pub found: usize,
    pub kept: usize,
    pub dropped: usize,
}

/// `POST /api/photos/:id/celestial-objects/recompute` — owner-only.
pub async fn recompute(
    State(state): State<AppState>,
    CurrentUser(viewer): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<RecomputeResponse>, AppError> {
    let owner_id: Uuid = sqlx::query_scalar("select owner_id from photos where id = $1")
        .bind(photo_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("photo".into()))?;
    if viewer.id != owner_id {
        return Err(AppError::Forbidden);
    }

    let mut tx = state.pool.begin().await?;
    let out = crate::celestial::identify(photo_id, &mut tx).await?;
    tx.commit().await?;
    Ok(Json(RecomputeResponse {
        found: out.found,
        kept: out.kept,
        dropped: out.dropped,
    }))
}
