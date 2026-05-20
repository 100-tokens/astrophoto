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
//!
//! Catalog v2 (migration 0022): the body MAY now carry structured
//! `brand` / `model` / `variant`. When present those take precedence:
//! display_name is regenerated as `<brand> <model>[ <variant>]` and
//! canonical_name follows the same shape lowercased. When absent the
//! handler falls back to the freetext path — brand="" and
//! model=trim(display_name) — so existing callers stay green.

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
    pub brand: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
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

    // Resolve (brand, model, variant, display, canonical) from the input.
    // Two branches:
    //   - structured: brand+model present → display/canonical regenerated.
    //   - freetext fallback: only display_name given → brand="" and
    //     model=trim(display_name); canonical via normalize_canonical so
    //     it matches what upsert.rs writes for verify-form rows.
    let (brand, model, variant_opt, display, canonical) = match (input.brand, input.model) {
        (Some(raw_brand), Some(raw_model)) => {
            let brand = raw_brand.trim().to_string();
            let model = raw_model.trim().to_string();
            if model.is_empty() {
                return Err(AppError::Validation("model is required".into()));
            }
            let variant_opt = input
                .variant
                .as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            let variant_suffix = variant_opt
                .as_ref()
                .map(|v| format!(" {v}"))
                .unwrap_or_default();
            // brand may legitimately be "" (unknown brand). Skip the
            // leading space in that case so display_name stays clean.
            let display = if brand.is_empty() {
                format!("{model}{variant_suffix}")
            } else {
                format!("{brand} {model}{variant_suffix}")
            };
            let canonical = crate::equipment::normalize_canonical(&display);
            (brand, model, variant_opt, display, canonical)
        }
        _ => {
            // Freetext fallback — back-compat with callers that only
            // know about display_name.
            let display = input.display_name.trim().to_string();
            if display.is_empty() {
                return Err(AppError::Validation("display_name is required".into()));
            }
            let canonical = crate::equipment::normalize_canonical(&display);
            (String::new(), display.clone(), None, display, canonical)
        }
    };

    // Validate specs kind before opening the transaction.
    if let Some(ref payload) = input.specs {
        specs::ensure_matches_kind(&input.kind, payload)?;
    }

    let mut tx = state.pool.begin().await?;

    let row = sqlx::query!(
        r#"
        with ins as (
            insert into equipment_items
                (kind, canonical_name, display_name, usage_count, submitted_by, approved_at,
                 brand, model, variant)
                 values ($1, $2, $3, 0, $4, now(), $5, $6, $7)
            on conflict (kind, canonical_name) do nothing
            returning id, kind, canonical_name, display_name, usage_count, brand, model, variant
        )
        select id as "id!", kind as "kind!", canonical_name as "canonical_name!",
               display_name as "display_name!", usage_count as "usage_count!",
               brand as "brand!", model as "model!", variant
          from ins
         union all
        select id, kind, canonical_name, display_name, usage_count, brand, model, variant
          from equipment_items
         where kind = $1 and canonical_name = $2
         limit 1
        "#,
        input.kind,
        canonical,
        display,
        user.0.id,
        brand,
        model,
        variant_opt.as_deref(),
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
        brand: row.brand,
        model: row.model,
        variant: row.variant,
    }))
}
