//! GET /api/equipment/setups/:id — full setup detail with item expansion.
//! Handler delegates to the `load()` helper used by create/update too.

use axum::{Json, extract::{Path, State}, response::IntoResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api_types::{EquipmentItemRef, SetupDetail, SetupItem};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let detail = load(&state.pool, user.0.id, id).await?;
    Ok(Json(detail))
}

pub async fn load(pool: &PgPool, owner_id: Uuid, id: Uuid) -> Result<SetupDetail, AppError> {
    let s = sqlx::query!(
        r#"select id, name, description, location, is_remote, is_default,
                  guiding, created_at, updated_at
             from equipment_setups
            where id = $1 and owner_id = $2"#,
        id, owner_id
    )
    .fetch_optional(pool).await?
    .ok_or_else(|| AppError::NotFound("setup not found".into()))?;

    let items = sqlx::query!(
        r#"select si.role,
                  ei.id, ei.kind, ei.canonical_name, ei.display_name
             from setup_items si
             join equipment_items ei on ei.id = si.item_id
            where si.setup_id = $1
            order by si.role, ei.canonical_name"#,
        id
    )
    .fetch_all(pool).await?;

    Ok(SetupDetail {
        id: s.id.to_string(),
        name: s.name,
        description: s.description,
        location: s.location,
        is_remote: s.is_remote,
        is_default: s.is_default,
        guiding: s.guiding,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
        items: items.into_iter().map(|r| SetupItem {
            role: r.role,
            item: EquipmentItemRef {
                id: r.id.to_string(),
                kind: r.kind,
                canonical_name: r.canonical_name,
                display_name: r.display_name,
            },
        }).collect(),
    })
}
