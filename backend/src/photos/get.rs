use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::OptionalUser;
use crate::http::AppState;
use crate::photos::queries::{self, PhotoRow};

#[derive(Serialize)]
pub struct PhotoDetail {
    pub id: String,
    pub owner_id: String,
    pub short_id: String,
    pub status: String,
    pub original_name: String,
    pub bytes: i64,
    pub mime: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    pub taken_at: Option<String>,
    pub created_at: String,
    pub appreciation_count: i64,
    pub comment_count: i64,
    pub is_draft: bool,
    pub last_step: Option<String>,
    pub replaced_at: Option<String>,
    pub original_uploaded_at: String,
    pub pipeline_error: Option<String>,
}

impl From<PhotoRow> for PhotoDetail {
    fn from(p: PhotoRow) -> Self {
        Self {
            id: p.id.to_string(),
            owner_id: p.owner_id.to_string(),
            short_id: p.short_id,
            status: p.status,
            original_name: p.original_name,
            bytes: p.bytes,
            mime: p.mime,
            width: p.width,
            height: p.height,
            camera: p.camera,
            lens: p.lens,
            iso: p.iso,
            exposure_s: p.exposure_s,
            focal_mm: p.focal_mm,
            target: p.target,
            caption: p.caption,
            taken_at: p.taken_at.map(|d| d.to_rfc3339()),
            created_at: p.created_at.to_rfc3339(),
            appreciation_count: 0,
            comment_count: 0,
            is_draft: p.published_at.is_none(),
            last_step: p.last_step,
            replaced_at: p.replaced_at.map(|d| d.to_rfc3339()),
            original_uploaded_at: p.original_uploaded_at.to_rfc3339(),
            pipeline_error: p.pipeline_error,
        }
    }
}

pub async fn handler(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PhotoDetail>, AppError> {
    let viewer = user.0.as_ref().map(|u| u.id);
    if !queries::is_visible_to(&state.pool, id, viewer).await? {
        return Err(AppError::not_found("photo"));
    }

    let row = queries::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::not_found("photo"))?;

    let appreciation_count = sqlx::query!(
        r#"select count(*) as "count!" from appreciations where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let comment_count = sqlx::query!(
        r#"select count(*) as "count!" from comments where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let row_owner = row.owner_id;
    let mut dto: PhotoDetail = row.into();
    dto.appreciation_count = appreciation_count;
    dto.comment_count = comment_count;
    // Hide pipeline_error from non-owners — it can carry internal diagnostic strings.
    if viewer != Some(row_owner) {
        dto.pipeline_error = None;
    }
    Ok(Json(dto))
}
