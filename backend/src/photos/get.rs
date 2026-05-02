use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::queries::{self, PhotoRow};

#[derive(Serialize)]
pub struct PhotoDetail {
    pub id: String,
    pub owner_id: String,
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
}

impl From<PhotoRow> for PhotoDetail {
    fn from(p: PhotoRow) -> Self {
        Self {
            id: p.id.to_string(),
            owner_id: p.owner_id.to_string(),
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
        }
    }
}

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PhotoDetail>, AppError> {
    let row = queries::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}
