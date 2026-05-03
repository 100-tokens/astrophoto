//! GET /api/tags/autocomplete?q=<query>
//!
//! Returns up to 10 tags whose slug or name matches the query
//! (case-insensitive). Public endpoint — no auth required.
//! Empty `q` returns an empty array immediately without touching the DB.

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub q: String,
}

#[derive(Serialize)]
pub struct Item {
    pub slug: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct R {
    pub tags: Vec<Item>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(qs): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let q = qs.q.trim();
    if q.is_empty() {
        return Ok(Json(R { tags: vec![] }));
    }
    let pattern = format!("%{q}%");
    let rows = sqlx::query!(
        "select slug, name from tags where slug ilike $1 or name ilike $1 order by slug limit 10",
        pattern
    )
    .fetch_all(&state.pool)
    .await?;

    let tags = rows
        .into_iter()
        .map(|r| Item {
            slug: r.slug,
            name: r.name,
        })
        .collect();
    Ok(Json(R { tags }))
}
