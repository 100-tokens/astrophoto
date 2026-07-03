use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{DiscoveryPhoto, SearchResults, SearchTargetHit, SearchUserHit};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub q: String,
}

const TARGET_CAP: i64 = 5;
const USER_CAP: i64 = 5;
const PHOTO_CAP: i64 = 24;

pub async fn get(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let term = q.q.trim();
    if term.is_empty() {
        return Err(AppError::bad_request("q_empty"));
    }
    let pattern = format!("%{}%", term.to_lowercase());

    // Targets: match canonical_name OR any alias (case-insensitive).
    let target_rows = sqlx::query!(
        r#"
        select t.slug as "slug!", t.canonical_name as "canonical_name!",
               (select count(*) from photo_targets pt join photos p on p.id = pt.photo_id
                where pt.target_id = t.id and p.published_at is not null and p.status='ready'
                  and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null))::int8 as "photo_count!"
        from targets t
        where t.canonical_name ilike $1
           or t.aliases_text ilike $1
        order by t.canonical_name
        limit $2
        "#,
        pattern,
        TARGET_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    let user_rows = sqlx::query!(
        r#"
        select u.id as "id!", u.handle as "handle!", u.display_name as "display_name!"
        from users u
        where (lower(u.handle) like $1 or lower(u.display_name) like $1)
          and u.pending_deletion_at is null
        order by u.handle
        limit $2
        "#,
        pattern,
        USER_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    struct PhotoRow {
        id: Uuid,
        short_id: String,
        target: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        blurhash: Option<String>,
        appreciations_count: i32,
        published_at: Option<chrono::DateTime<chrono::Utc>>,
        owner_id: Uuid,
        handle: String,
        display_name: String,
    }

    let photo_rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select p.id as "id!", p.short_id as "short_id!", p.target,
               p.width, p.height, p.blurhash,
               p.appreciations_count as "appreciations_count!",
               p.published_at,
               u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
        from photos p
        join users u on u.id = p.owner_id
        where p.published_at is not null and p.status = 'ready'
          and u.pending_deletion_at is null
          and (lower(coalesce(p.target, '')) like $1 or lower(coalesce(p.caption, '')) like $1)
        order by p.published_at desc, p.id desc
        limit $2
        "#,
        pattern,
        PHOTO_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(SearchResults {
        q: term.to_string(),
        targets: target_rows
            .into_iter()
            .map(|r| SearchTargetHit {
                slug: r.slug,
                canonical_name: r.canonical_name,
                photo_count: r.photo_count,
            })
            .collect(),
        users: user_rows
            .into_iter()
            .map(|r| SearchUserHit {
                id: r.id,
                handle: r.handle,
                display_name: r.display_name,
            })
            .collect(),
        photos: photo_rows
            .into_iter()
            .map(|r| DiscoveryPhoto {
                id: r.id,
                short_id: r.short_id,
                target: r.target,
                original_name: None,
                width: r.width,
                height: r.height,
                blurhash: r.blurhash,
                appreciations_count: r.appreciations_count,
                published_at: r.published_at,
                author_id: r.owner_id,
                author_handle: r.handle,
                author_display_name: r.display_name,
            })
            .collect(),
    }))
}
