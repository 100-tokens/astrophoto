//! Super-admin equipment catalog management.
//!
//!   * `GET    /api/admin/equipment`      — list all items (every kind/status),
//!     paginated, optional `kind` + `q` (free-text) filters.
//!   * `PATCH  /api/admin/equipment/:id`  — edit brand/model/variant/display_name
//!     (regenerates `canonical_name`).
//!   * `DELETE /api/admin/equipment/:id`  — delete an ORPHANED item only
//!     (refused if any photo or setup still references it).
//!
//! Merge/dedup of duplicates is intentionally NOT implemented here: it would
//! rewrite denormalized freetext gear columns across `photos`, which is an
//! irreversible data-mutation with no safe partial state. It ships separately.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{AdminEquipmentItem, AdminEquipmentPage};
use crate::auth::middleware::AdminUser;
use crate::http::AppState;

const PAGE_SIZE: i64 = 50;

#[derive(Deserialize)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub q: Option<String>,
    pub page: Option<i32>,
}

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = query.page.unwrap_or(0).max(0);
    let offset = page as i64 * PAGE_SIZE;
    let kind = query.kind.filter(|k| !k.is_empty());
    let search = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"));

    let total: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!"
             from equipment_items
            where ($1::text is null or kind = $1)
              and ($2::text is null
                   or display_name ilike $2 or canonical_name ilike $2
                   or brand ilike $2 or model ilike $2)"#,
        kind.as_deref(),
        search.as_deref(),
    )
    .fetch_one(&state.pool)
    .await?;

    let rows = sqlx::query!(
        r#"select e.id, e.kind, e.brand, e.model, e.variant, e.display_name,
                  e.canonical_name, e.usage_count, e.status, e.created_at,
                  u.handle::text as submitted_by_handle
             from equipment_items e
             left join users u on u.id = e.submitted_by
            where ($1::text is null or e.kind = $1)
              and ($2::text is null
                   or e.display_name ilike $2 or e.canonical_name ilike $2
                   or e.brand ilike $2 or e.model ilike $2)
            order by e.usage_count desc, e.created_at desc
            limit $3 offset $4"#,
        kind.as_deref(),
        search.as_deref(),
        PAGE_SIZE,
        offset,
    )
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<AdminEquipmentItem> = rows
        .into_iter()
        .map(|r| AdminEquipmentItem {
            id: r.id,
            kind: r.kind,
            brand: r.brand,
            model: r.model,
            variant: r.variant,
            display_name: r.display_name,
            canonical_name: r.canonical_name,
            usage_count: r.usage_count,
            status: r.status,
            submitted_by_handle: r.submitted_by_handle,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    let has_more = offset + (items.len() as i64) < total;
    Ok(Json(AdminEquipmentPage {
        items,
        total,
        page,
        has_more,
    }))
}

#[derive(Deserialize)]
pub struct EditBody {
    pub brand: Option<String>,
    pub model: Option<String>,
    /// Provided (non-empty) sets the variant; provided empty clears it; absent leaves it.
    pub variant: Option<String>,
    pub display_name: Option<String>,
}

pub async fn edit(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(body): Json<EditBody>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    let row = sqlx::query!(
        r#"select brand, model, variant, display_name
             from equipment_items where id = $1 for update"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("equipment item"))?;

    let brand = body
        .brand
        .map(|s| s.trim().to_string())
        .unwrap_or(row.brand);
    let model = body
        .model
        .map(|s| s.trim().to_string())
        .unwrap_or(row.model);
    let variant = match body.variant {
        Some(v) => {
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        None => row.variant,
    };
    let display_name = body
        .display_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(row.display_name);

    if model.is_empty() {
        return Err(AppError::Validation("model cannot be empty".into()));
    }
    let canonical = crate::equipment::normalize_canonical(&display_name);

    let res = sqlx::query!(
        r#"update equipment_items
              set brand = $1, model = $2, variant = $3,
                  display_name = $4, canonical_name = $5
            where id = $6"#,
        brand,
        model,
        variant,
        display_name,
        canonical,
        id,
    )
    .execute(&mut *tx)
    .await;

    match res {
        Ok(_) => {}
        Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
            return Err(AppError::Conflict(
                "another item of this kind already uses that name".into(),
            ));
        }
        Err(e) => return Err(AppError::Database(e)),
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // An item is safe to delete only when nothing references it: zero usage,
    // not in any setup, not on any photo's filter junction. (Specs sub-table
    // rows cascade automatically.) setup_items / photo_filters are ON DELETE
    // RESTRICT, so a referenced delete would error anyway — we pre-check for
    // a clean 409.
    let row = sqlx::query!(
        r#"select e.usage_count,
                  (select count(*) from setup_items s where s.item_id = e.id)   as "setups!",
                  (select count(*) from photo_filters f where f.item_id = e.id) as "filters!"
             from equipment_items e where e.id = $1"#,
        id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("equipment item"))?;

    if row.usage_count > 0 || row.setups > 0 || row.filters > 0 {
        return Err(AppError::Conflict(
            "item is still in use by photos or setups; cannot delete".into(),
        ));
    }

    sqlx::query!("delete from equipment_items where id = $1", id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
