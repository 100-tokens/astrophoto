use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

const MAX_SLOTS: i64 = 6;

pub async fn pin(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    // Verify ownership + status, lock the row.
    let photo = sqlx::query!(
        r#"
        select featured_position
        from photos
        where id = $1
          and owner_id = $2
          and published_at is not null
          and status = 'ready'
        for update
        "#,
        photo_id,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(p) = photo else {
        return Err(AppError::not_found("photo_not_owned_or_unpublished"));
    };
    if p.featured_position.is_some() {
        // Idempotent.
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    // Find the lowest free position 1..=6. If none, 409.
    let row = sqlx::query!(
        r#"
        select coalesce(min(p), 0)::int8 as next_pos
        from generate_series(1, $1::int8) as g(p)
        where not exists (
            select 1 from photos
            where owner_id = $2
              and featured_position = g.p
              and featured_at is not null
        )
        "#,
        MAX_SLOTS,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;
    let next = row.next_pos.unwrap_or(0);
    if next == 0 {
        return Err(AppError::Conflict("featured_full".into()));
    }

    sqlx::query!(
        "update photos set featured_at = now(), featured_position = $1 where id = $2",
        next as i16,
        photo_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unpin(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    let row = sqlx::query!(
        r#"
        update photos
           set featured_at = null, featured_position = null
         where id = $1
           and owner_id = $2
           and featured_at is not null
        returning featured_position
        "#,
        photo_id,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(r) = row {
        let removed = r.featured_position.unwrap_or(1) as i64;
        // Compact: every position > removed shifts down by 1.
        // Stage via NULL first (both fields) to avoid the partial unique index
        // and the photos_featured_pair_chk constraint tripping mid-transaction.
        let to_shift = sqlx::query!(
            r#"
            select id, featured_position from photos
            where owner_id = $1 and featured_at is not null and featured_position > $2
            order by featured_position
            for update
            "#,
            user.id,
            removed as i16
        )
        .fetch_all(&mut *tx)
        .await?;

        // Pass 1: fully unpin all rows that need to shift (both fields NULL —
        // required by photos_featured_pair_chk).
        for r in &to_shift {
            sqlx::query!(
                "update photos set featured_at = null, featured_position = null where id = $1",
                r.id
            )
            .execute(&mut *tx)
            .await?;
        }
        // Pass 2: restore with new positions.
        for r in &to_shift {
            let new_pos = r.featured_position.unwrap_or(1) - 1;
            sqlx::query!(
                "update photos set featured_at = now(), featured_position = $1 where id = $2",
                new_pos,
                r.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReorderBody {
    pub photo_ids: Vec<Uuid>,
}

pub async fn reorder(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.photo_ids.is_empty() || body.photo_ids.len() > MAX_SLOTS as usize {
        return Err(AppError::bad_request("featured_order_count"));
    }
    let mut seen = std::collections::HashSet::new();
    for id in &body.photo_ids {
        if !seen.insert(*id) {
            return Err(AppError::bad_request("featured_order_duplicate"));
        }
    }

    let mut tx = state.pool.begin().await?;

    // Verify every supplied id is owned + currently pinned.
    let owned = sqlx::query!(
        r#"
        select id from photos
        where owner_id = $1 and featured_at is not null and id = any($2)
        for update
        "#,
        user.id,
        &body.photo_ids
    )
    .fetch_all(&mut *tx)
    .await?;
    if owned.len() != body.photo_ids.len() {
        return Err(AppError::bad_request("featured_order_unknown_id"));
    }

    // Pass 1: stage all affected rows fully unpinned (both fields NULL —
    // required by photos_featured_pair_chk and the partial unique index).
    sqlx::query!(
        "update photos set featured_at = null, featured_position = null where id = any($1)",
        &body.photo_ids
    )
    .execute(&mut *tx)
    .await?;

    // Pass 2: write target positions, restore featured_at = now().
    for (i, id) in body.photo_ids.iter().enumerate() {
        let pos = (i as i16) + 1;
        sqlx::query!(
            "update photos set featured_at = now(), featured_position = $1 where id = $2",
            pos,
            id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
