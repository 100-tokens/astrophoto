//! POST /api/equipment/setups — create a setup with its items.
//! Default-exclusivity enforced in the same transaction (clear-others
//! before insert) so the partial unique idx never trips.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

use crate::api_types::SetupInput;
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

use super::{unique_conflict_to_422, unknown_item_to_422, validate_role};

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
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

    if input.is_default {
        sqlx::query!(
            "update equipment_setups set is_default = false
             where owner_id = $1 and is_default",
            user.0.id
        )
        .execute(&mut *tx)
        .await?;
    }

    let row = sqlx::query!(
        r#"insert into equipment_setups
            (owner_id, name, description, location, is_remote, is_default, guiding)
            values ($1,$2,$3,$4,$5,$6,$7)
            returning id"#,
        user.0.id, input.name.trim(),
        input.description.as_deref(), input.location.as_deref(),
        input.is_remote, input.is_default, input.guiding.as_deref()
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(unique_conflict_to_422)?;
    let setup_id = row.id;

    for (i, it) in input.items.iter().enumerate() {
        sqlx::query!(
            "insert into setup_items (setup_id, role, item_id) values ($1,$2,$3)",
            setup_id, it.role, item_uuids[i]
        )
        .execute(&mut *tx)
        .await
        .map_err(unknown_item_to_422)?;
    }

    tx.commit().await?;

    let detail = super::get::load(&state.pool, user.0.id, setup_id).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}
