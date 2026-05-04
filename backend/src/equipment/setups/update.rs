//! PATCH /api/equipment/setups/:id — meta + items replace-all.

use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;

use crate::api_types::SetupInput;
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

use super::{unique_conflict_to_422, unknown_item_to_422, validate_role};

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<SetupInput>,
) -> Result<impl IntoResponse, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let mut item_uuids = Vec::with_capacity(input.items.len());
    for it in &input.items {
        validate_role(&it.role)?;
        let uuid = Uuid::parse_str(&it.item_id)
            .map_err(|_| AppError::Validation("item_id is not a uuid".into()))?;
        item_uuids.push(uuid);
    }

    let mut tx = state.pool.begin().await?;

    // Confirm ownership and lock the row.
    let exists = sqlx::query_scalar!(
        "select id from equipment_setups where id = $1 and owner_id = $2 for update",
        id, user.0.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("setup not found".into()));
    }

    if input.is_default {
        sqlx::query!(
            "update equipment_setups set is_default = false
             where owner_id = $1 and is_default and id <> $2",
            user.0.id, id
        )
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query!(
        r#"update equipment_setups
              set name=$1, description=$2, location=$3,
                  is_remote=$4, is_default=$5, guiding=$6,
                  updated_at=now()
            where id=$7"#,
        input.name.trim(), input.description.as_deref(), input.location.as_deref(),
        input.is_remote, input.is_default, input.guiding.as_deref(), id
    )
    .execute(&mut *tx)
    .await
    .map_err(unique_conflict_to_422)?;

    sqlx::query!("delete from setup_items where setup_id=$1", id)
        .execute(&mut *tx).await?;

    for (i, it) in input.items.iter().enumerate() {
        sqlx::query!(
            "insert into setup_items (setup_id, role, item_id) values ($1,$2,$3)",
            id, it.role, item_uuids[i]
        )
        .execute(&mut *tx)
        .await
        .map_err(unknown_item_to_422)?;
    }

    tx.commit().await?;

    Ok(Json(super::get::load(&state.pool, user.0.id, id).await?))
}
