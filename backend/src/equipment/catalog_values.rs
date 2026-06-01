//! GET /api/equipment/catalog-values?kind=<kind>[&brand=<b>][&model=<m>]
//!
//! Returns the distinct catalog values that power the assisted,
//! duplicate-avoiding brand/model/variant pickers (a cascading LOV):
//! `brands` is the distinct non-empty brands for the kind (always);
//! `models` the distinct models for (kind, brand), only when `brand` is a
//! non-empty string; `variants` the distinct variants for (kind, brand,
//! model), only when both `brand` and `model` are non-empty.
//!
//! Each value carries the count of items currently using it (for the "· 12"
//! usage hint + most-used-first ordering). Rows with `status = 'merged'` are
//! excluded. Public endpoint — no auth (catalog metadata).
//!
//! Note: a `brand = ''` (the unknown-brand sentinel from the migration-0022
//! backfill) is treated as "no brand", so it never fires the meaningless
//! "models for the empty brand" query.

use axum::{Json, extract::Query, extract::State, response::IntoResponse};
use serde::Deserialize;

use crate::api_types::{CatalogValue, CatalogValues};
use crate::equipment::VALID_KINDS;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub kind: String,
    pub brand: Option<String>,
    pub model: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    if !VALID_KINDS.contains(&q.kind.as_str()) {
        return Err(AppError::Validation(format!("unknown kind: {}", q.kind)));
    }
    let brand = q
        .brand
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let model = q
        .model
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let brands = sqlx::query!(
        r#"select brand as "value!", count(*)::int4 as "count!"
             from equipment_items
            where kind = $1 and brand <> '' and status <> 'merged'
            group by brand
            order by count(*) desc, brand"#,
        q.kind,
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| CatalogValue {
        value: r.value,
        count: r.count,
    })
    .collect();

    let models = if let Some(ref b) = brand {
        sqlx::query!(
            r#"select model as "value!", count(*)::int4 as "count!"
                 from equipment_items
                where kind = $1 and brand = $2 and status <> 'merged'
                group by model
                order by count(*) desc, model"#,
            q.kind,
            b,
        )
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|r| CatalogValue {
            value: r.value,
            count: r.count,
        })
        .collect()
    } else {
        Vec::new()
    };

    let variants = if let (Some(b), Some(m)) = (&brand, &model) {
        sqlx::query!(
            r#"select variant as "value!", count(*)::int4 as "count!"
                 from equipment_items
                where kind = $1 and brand = $2 and model = $3
                  and variant is not null and variant <> '' and status <> 'merged'
                group by variant
                order by count(*) desc, variant"#,
            q.kind,
            b,
            m,
        )
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|r| CatalogValue {
            value: r.value,
            count: r.count,
        })
        .collect()
    } else {
        Vec::new()
    };

    Ok(Json(CatalogValues {
        brands,
        models,
        variants,
    }))
}
