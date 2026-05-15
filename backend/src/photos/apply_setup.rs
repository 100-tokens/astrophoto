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

use crate::api_types::{ApplySetupInput, PhotoFilterChip};
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
    /// Typed filter chips after apply — read from the junction so the
    /// frontend can refresh `FilterChipInput` without a follow-up
    /// `GET /api/photos/:id`. Empty when no filters apply.
    pub filter_items: Vec<PhotoFilterChip>,
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

    // Resolve canonical names and item ids from setup_items.
    let items = sqlx::query!(
        r#"select si.role, ei.id as item_id, ei.display_name, ei.canonical_name
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
    // filter_pairs: (display_name, item_id) — sorted alphabetically by display_name below.
    let mut filter_pairs: Vec<(String, Uuid)> = vec![];
    for r in items {
        match r.role.as_str() {
            "optical_tube" => scope = Some(r.display_name),
            "focal_modifier" => focal_mod = Some(r.display_name),
            "main_camera" => camera = Some(r.display_name),
            "mount" => mount = Some(r.display_name),
            "filter" => filter_pairs.push((r.display_name, r.item_id)),
            other => {
                tracing::warn!(role = %other, "unknown setup_items role in apply-setup; ignored")
            }
        }
    }
    // Sort alphabetically by display_name so cache string and junction positions agree.
    filter_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let filters = if filter_pairs.is_empty() {
        None
    } else {
        Some(
            filter_pairs
                .iter()
                .map(|p| p.0.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };
    let guiding = setup.guiding;

    // Determine whether the photo_filters junction needs syncing.
    // overwrite → always sync (delete existing rows first, re-insert).
    // fill_empty → sync only when both the junction is empty AND filters cache is null/empty.
    let do_junction_sync = if mode_overwrite {
        true
    } else {
        let junction_empty = sqlx::query_scalar!(
            "select not exists(select 1 from photo_filters where photo_id=$1)",
            photo_id
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(true);
        let cache_empty = sqlx::query_scalar!(
            "select coalesce(filters,'') = '' from photos where id=$1",
            photo_id
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(true);
        junction_empty && cache_empty
    };

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

    // Sync photo_filters junction when appropriate.
    // In overwrite mode the delete runs unconditionally so stale junction rows
    // from a previous setup don't linger even when the new setup has no filters.
    if do_junction_sync {
        sqlx::query!("delete from photo_filters where photo_id = $1", photo_id)
            .execute(&mut *tx)
            .await?;
        if !filter_pairs.is_empty() {
            for (i, (_, item_id)) in filter_pairs.iter().enumerate() {
                sqlx::query!(
                    "insert into photo_filters (photo_id, item_id, position) values ($1,$2,$3)",
                    photo_id,
                    item_id,
                    i as i16
                )
                .execute(&mut *tx)
                .await?;
            }
            // Rebuild overwrites photos.filters with the junction-derived string;
            // junction is source of truth for the cache.
            crate::photos::filters_cache::rebuild(&mut tx, photo_id).await?;
        }
    }

    tx.commit().await?;

    // Read the typed chips from the now-final junction. One round-trip
    // outside the tx; the caller has just observed the write so this
    // never reflects a stale state.
    let filter_items: Vec<PhotoFilterChip> = sqlx::query!(
        r#"select pf.item_id, pf.position, e.display_name as "display_name!",
                  fs.filter_type, fs.bandwidth_nm
             from photo_filters pf
             join equipment_items e on e.id = pf.item_id
        left join filter_specs fs on fs.item_id = pf.item_id
            where pf.photo_id = $1
            order by pf.position, e.display_name"#,
        photo_id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| PhotoFilterChip {
        id: r.item_id.to_string(),
        display_name: r.display_name,
        filter_type: r
            .filter_type
            .and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok()),
        bandwidth_nm: r.bandwidth_nm.and_then(|n| n.to_string().parse::<f64>().ok()),
        position: r.position as i32,
    })
    .collect();

    Ok(Json(AppliedOut {
        setup_id: updated.setup_id.map(|u| u.to_string()),
        scope: updated.scope,
        focal_modifier: updated.focal_modifier,
        camera: updated.camera,
        mount: updated.mount,
        filters: updated.filters,
        guiding: updated.guiding,
        filter_items,
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
