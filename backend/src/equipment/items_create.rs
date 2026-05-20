//! POST /api/equipment/items
//!
//! Resolve-or-create one canonical equipment item. Returns the existing
//! row on hit (no usage_count bump — that counter remains photo-save
//! driven via `crate::equipment::upsert`) or inserts on miss with
//! usage_count = 0 and submitted_by = calling user. Authenticated users
//! only.
//!
//! If `specs` is provided in the body:
//!   1. `ensure_matches_kind` validates that the payload kind matches the
//!      item kind (422 on mismatch).
//!   2. All DB writes run inside a single transaction.
//!   3. Any prior `<kind>_specs` row for this item_id is deleted before
//!      the fresh INSERT, giving PUT-semantics on the embedded specs.

use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use crate::api_types::EquipmentItemInput;
use crate::auth::middleware::CurrentUser;
use crate::equipment::{VALID_KINDS, specs};
use crate::error::AppError;
use crate::http::AppState;

#[derive(Serialize)]
pub struct Out {
    pub id: String,
    pub kind: String,
    pub canonical_name: String,
    pub display_name: String,
    pub usage_count: i32,
}

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<EquipmentItemInput>,
) -> Result<impl IntoResponse, AppError> {
    if !VALID_KINDS.contains(&input.kind.as_str()) {
        return Err(AppError::Validation(
            "kind must be telescope|camera|mount|filter|focal_modifier|guiding".into(),
        ));
    }
    let display = input.display_name.trim();
    if display.is_empty() {
        return Err(AppError::Validation("display_name is required".into()));
    }
    let canonical = crate::equipment::normalize_canonical(display);

    // Validate specs kind before opening the transaction.
    if let Some(ref payload) = input.specs {
        specs::ensure_matches_kind(&input.kind, payload)?;
    }

    let mut tx = state.pool.begin().await?;

    let row = sqlx::query!(
        r#"
        with ins as (
            insert into equipment_items
                (kind, canonical_name, display_name, usage_count, submitted_by, approved_at)
                 values ($1, $2, $3, 0, $4, now())
            on conflict (kind, canonical_name) do nothing
            returning id, kind, canonical_name, display_name, usage_count
        )
        select id as "id!", kind as "kind!", canonical_name as "canonical_name!",
               display_name as "display_name!", usage_count as "usage_count!"
          from ins
         union all
        select id, kind, canonical_name, display_name, usage_count
          from equipment_items
         where kind = $1 and canonical_name = $2
         limit 1
        "#,
        input.kind,
        canonical,
        display,
        user.0.id,
    )
    .fetch_one(&mut *tx)
    .await?;

    if let Some(ref payload) = input.specs {
        specs::delete_specs_row(&mut tx, row.id, &row.kind).await?;
        specs::insert_specs_row(&mut tx, row.id, payload).await?;
    }

    tx.commit().await?;

    Ok(Json(Out {
        id: row.id.to_string(),
        kind: row.kind,
        canonical_name: row.canonical_name,
        display_name: row.display_name,
        usage_count: row.usage_count,
    }))
}
