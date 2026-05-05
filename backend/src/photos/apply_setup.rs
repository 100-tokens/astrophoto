//! POST /api/photos/:id/apply-setup       { setup_id, mode }
//! POST /api/photos/:id/detach-setup
//!
//! Two handlers in one file. mode = "fill_empty" | "overwrite".
//! In both modes, setup_id on the photo is set unconditionally — the FK
//! records origin, not equality with the setup's current state.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::api_types::ApplySetupInput;
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Serialize)]
pub struct AppliedOut {
    pub setup_id: Option<String>,
    pub scope: Option<String>,
    pub focal_modifier: Option<String>,
    pub camera: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
}

pub async fn apply(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(photo_id): Path<Uuid>,
    Json(input): Json<ApplySetupInput>,
) -> Result<impl IntoResponse, AppError> {
    let setup_uuid = Uuid::parse_str(&input.setup_id)
        .map_err(|_| AppError::Validation("setup_id is not a uuid".into()))?;
    let mode_overwrite = match input.mode.as_str() {
        "fill_empty" => false,
        "overwrite" => true,
        _ => {
            return Err(AppError::Validation(
                "mode must be 'fill_empty' or 'overwrite'".into(),
            ));
        }
    };

    let mut tx = state.pool.begin().await?;

    // Confirm setup belongs to caller. 404 if not.
    let setup = sqlx::query!(
        "select guiding from equipment_setups where id=$1 and owner_id=$2",
        setup_uuid,
        user.0.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("setup not found".into()))?;

    // Confirm photo belongs to caller AND lock the row.
    let owns_photo = sqlx::query_scalar!(
        "select id from photos where id=$1 and owner_id=$2 for update",
        photo_id,
        user.0.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if owns_photo.is_none() {
        return Err(AppError::NotFound("photo not found".into()));
    }

    // Resolve canonical names from setup_items.
    let items = sqlx::query!(
        r#"select si.role, ei.display_name, ei.canonical_name
             from setup_items si
             join equipment_items ei on ei.id = si.item_id
            where si.setup_id = $1
            order by si.role, ei.canonical_name"#,
        setup_uuid
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut scope: Option<String> = None;
    let mut focal_mod: Option<String> = None;
    let mut camera: Option<String> = None;
    let mut mount: Option<String> = None;
    let mut filters_buf: Vec<String> = vec![];
    for r in items {
        match r.role.as_str() {
            "optical_tube" => scope = Some(r.display_name),
            "focal_modifier" => focal_mod = Some(r.display_name),
            "main_camera" => camera = Some(r.display_name),
            "mount" => mount = Some(r.display_name),
            "filter" => filters_buf.push(r.display_name),
            other => {
                tracing::warn!(role = %other, "unknown setup_items role in apply-setup; ignored")
            }
        }
    }
    let filters = if filters_buf.is_empty() {
        None
    } else {
        Some(filters_buf.join(", "))
    };
    let guiding = setup.guiding;

    // The CASE expression handles both modes via the $2 boolean:
    //   - mode_overwrite=true: always write the new value.
    //   - mode_overwrite=false: only write if the current column is NULL or empty.
    let updated = sqlx::query!(
        r#"
        update photos
           set scope          = case when $2::bool or scope is null
                                          or scope = '' then $3 else scope end,
               focal_modifier = case when $2::bool or focal_modifier is null
                                          or focal_modifier = '' then $4 else focal_modifier end,
               camera         = case when $2::bool or camera is null
                                          or camera = '' then $5 else camera end,
               mount          = case when $2::bool or mount is null
                                          or mount = '' then $6 else mount end,
               filters        = case when $2::bool or filters is null
                                          or filters = '' then $7 else filters end,
               guiding        = case when $2::bool or guiding is null
                                          or guiding = '' then $8 else guiding end,
               setup_id       = $9
         where id = $1
       returning setup_id, scope, focal_modifier, camera, mount, filters, guiding
        "#,
        photo_id,
        mode_overwrite,
        scope,
        focal_mod,
        camera,
        mount,
        filters,
        guiding,
        setup_uuid
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(AppliedOut {
        setup_id: updated.setup_id.map(|u| u.to_string()),
        scope: updated.scope,
        focal_modifier: updated.focal_modifier,
        camera: updated.camera,
        mount: updated.mount,
        filters: updated.filters,
        guiding: updated.guiding,
    }))
}

pub async fn detach(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let res = sqlx::query!(
        "update photos set setup_id=null where id=$1 and owner_id=$2",
        photo_id,
        user.0.id
    )
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("photo not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
