//! GET /api/equipment/autocomplete?kind=<kind>&q=<query>
//!
//! Returns up to 10 equipment_items rows for the given kind, matching ILIKE
//! on canonical_name OR display_name, ordered by usage_count DESC.
//! Public endpoint — no auth required.
//! Empty `q` returns an empty array immediately without touching the DB.
//! Invalid `kind` returns 422 Validation.

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http::AppState;

const VALID_KINDS: &[&str] = &[
    "telescope",
    "camera",
    "mount",
    "filter",
    "focal_modifier",
    "guiding",
];

#[derive(Deserialize)]
pub struct Q {
    pub kind: String,
    pub q: String,
}

#[derive(Serialize)]
pub struct Item {
    pub canonical_name: String,
    pub display_name: String,
    pub usage_count: i32,
}

#[derive(Serialize)]
pub struct R {
    pub items: Vec<Item>,
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
    let q = qs.q.trim();
    if q.is_empty() {
        return Ok(Json(R { items: vec![] }));
    }
    let pattern = format!("%{q}%");
    let rows = sqlx::query!(
        r#"
        select canonical_name, display_name, usage_count
          from equipment_items
         where kind = $1
           and (canonical_name ilike $2 or display_name ilike $2)
         order by usage_count desc
         limit 10
        "#,
        qs.kind,
        pattern
    )
    .fetch_all(&state.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| Item {
            canonical_name: r.canonical_name,
            display_name: r.display_name,
            usage_count: r.usage_count,
        })
        .collect();
    Ok(Json(R { items }))
}
