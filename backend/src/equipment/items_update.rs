//! PATCH /api/equipment/items/:id
//!
//! Updates `display_name` (and derived `canonical_name`) and/or fully
//! replaces the per-kind specs sub-table row for an equipment item.
//! Both fields are optional; omitting both is a no-op that still returns
//! the current item.
//!
//! Specs replacement is atomic: the old sub-table row is deleted and a
//! fresh one inserted inside the same transaction. If kind validation
//! fails (422) the rename is also rolled back.

use axum::{Json, extract::Path, extract::State, response::IntoResponse};
use uuid::Uuid;

use crate::api_types::EquipmentItemPatch;
use crate::auth::middleware::CurrentUser;
use crate::equipment::specs;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<EquipmentItemPatch>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    // Lock the row and fetch its kind. 404 if not found.
    let row = sqlx::query!(
        r#"select kind from equipment_items where id = $1 for update"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("equipment item not found".into()))?;

    // Validate specs kind before writing anything, so a 422 rolls back clean.
    if let Some(ref payload) = input.specs {
        specs::ensure_matches_kind(&row.kind, payload)?;
    }

    if let Some(ref raw_name) = input.display_name {
        let display = raw_name.trim();
        if display.is_empty() {
            return Err(AppError::Validation("display_name cannot be empty".into()));
        }
        let canonical = display.to_lowercase();
        sqlx::query!(
            r#"update equipment_items
                  set display_name = $1, canonical_name = $2
                where id = $3"#,
            display,
            canonical,
            id,
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(ref payload) = input.specs {
        specs::delete_specs_row(&mut tx, id, &row.kind).await?;
        specs::insert_specs_row(&mut tx, id, payload).await?;
    }

    tx.commit().await?;

    // Re-fetch via items_get so the response is always consistent.
    crate::equipment::items_get::handler(State(state), _user, Path(id)).await
}
