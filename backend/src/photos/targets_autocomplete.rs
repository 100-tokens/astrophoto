//! GET /api/targets/autocomplete?q=<query>
//!
//! Returns up to 10 targets whose slug, canonical_name, or any alias
//! matches the query (case-insensitive). Public endpoint — no auth required.
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
    pub canonical_name: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct R {
    pub targets: Vec<Item>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(qs): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let q = qs.q.trim();
    if q.is_empty() {
        return Ok(Json(R { targets: vec![] }));
    }
    let pattern = format!("%{q}%");
    let rows = sqlx::query!(
        r#"
        select slug, canonical_name, kind
          from targets
         where slug ilike $1
            or canonical_name ilike $1
            or exists (select 1 from unnest(aliases) a where a ilike $1)
         order by slug
         limit 10
        "#,
        pattern
    )
    .fetch_all(&state.pool)
    .await?;

    let targets = rows
        .into_iter()
        .map(|r| Item {
            slug: r.slug,
            canonical_name: r.canonical_name,
            kind: r.kind,
        })
        .collect();
    Ok(Json(R { targets }))
}
