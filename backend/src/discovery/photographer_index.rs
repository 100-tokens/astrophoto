//! GET /api/photographers — paginated index of photographers, ordered by
//! one of: active (frame count), followers, recent (member-since).

use axum::{
    Json,
    extract::{Query, State},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Datelike;
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{PhotographerIndexPage, PhotographerListItem};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct ListQ {
    pub sort: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

// One cursor shape per sort. The integer is the sort key tiebreaker
// (frames / followers / nothing for recent — recent uses created_at).
// The `sort` tag rejects cross-sort replay: active and followers share
// the CountCursor shape, so an untagged cursor minted under one sort
// silently paginated the other from a wrong keyset position.
#[derive(serde::Serialize, serde::Deserialize)]
struct CountCursor {
    sort: String,
    count: i64,
    id: Uuid,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct DateCursor {
    sort: String,
    created_at: chrono::DateTime<chrono::Utc>,
    id: Uuid,
}

fn encode<T: serde::Serialize>(c: &T) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Strict decode: a malformed cursor is a caller error, not a silent
/// page-1 restart (which fed duplicate rows to keyed clients). Matches
/// the shared discovery/cursor.rs contract.
fn decode<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, AppError> {
    URL_SAFE_NO_PAD
        .decode(s)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| AppError::bad_request("cursor_invalid"))
}

struct Row {
    id: Uuid,
    handle: String,
    display_name: String,
    frame_count: i64,
    follower_count: i64,
    integration_seconds: i64,
    cover_photo_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Row> for PhotographerListItem {
    fn from(r: Row) -> Self {
        PhotographerListItem {
            handle: r.handle,
            display_name: r.display_name,
            frame_count: r.frame_count,
            follower_count: r.follower_count,
            integration_seconds: r.integration_seconds,
            cover_photo_id: r.cover_photo_id.map(|id| id.to_string()),
            member_since_year: r.created_at.year(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<PhotographerIndexPage>, AppError> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    // Unknown sorts are 400s, consistent with the explore contract —
    // they used to silently serve the 'active' feed.
    let sort = match q.sort.as_deref() {
        None | Some("active") => "active",
        Some("followers") => "followers",
        Some("recent") => "recent",
        Some(_) => return Err(AppError::bad_request("sort_invalid")),
    };

    // Per-sort branch — single SQL query each, ordered by sort key + id
    // tiebreaker so the cursor is deterministic across ties.
    let rows = match sort {
        "followers" => {
            let cur = q.cursor.as_deref().map(decode::<CountCursor>).transpose()?;
            if cur.as_ref().is_some_and(|c| c.sort != "followers") {
                return Err(AppError::bad_request("cursor_invalid"));
            }
            sqlx::query_as!(
                Row,
                r#"
                with photo_stats as (
                  -- Pre-aggregated per owner: joining photos AND follows
                  -- directly onto users fans out to photos × follows rows
                  -- and multiplies the integration sum by follower count.
                  select
                    owner_id,
                    count(*)::bigint as frame_count,
                    coalesce(sum(coalesce(integration_s, exposure_s * coalesce(sessions, 1))), 0)::bigint as integration_seconds
                  from photos
                  where published_at is not null and status = 'ready'
                  group by owner_id
                ),
                stats as (
                  select
                    u.id,
                    u.handle::text as handle,
                    u.display_name,
                    u.created_at,
                    u.cover_photo_id,
                    coalesce(ps.frame_count, 0)::bigint as frame_count,
                    coalesce(ps.integration_seconds, 0)::bigint as integration_seconds,
                    coalesce(count(distinct f.follower_id), 0)::bigint as follower_count
                  from users u
                  left join photo_stats ps on ps.owner_id = u.id
                  left join follows f on f.followed_id = u.id
                  group by u.id, ps.frame_count, ps.integration_seconds
                )
                select
                  id as "id!",
                  handle as "handle!",
                  display_name as "display_name!",
                  created_at as "created_at!",
                  cover_photo_id,
                  frame_count as "frame_count!",
                  integration_seconds as "integration_seconds!",
                  follower_count as "follower_count!"
                  from stats
                 where frame_count > 0
                   and ($1::bigint is null or follower_count < $1
                        or (follower_count = $1 and id > $2::uuid))
                 order by follower_count desc, id asc
                 limit $3
                "#,
                cur.as_ref().map(|c| c.count),
                cur.as_ref().map(|c| c.id),
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        "recent" => {
            let cur = q.cursor.as_deref().map(decode::<DateCursor>).transpose()?;
            if cur.as_ref().is_some_and(|c| c.sort != "recent") {
                return Err(AppError::bad_request("cursor_invalid"));
            }
            sqlx::query_as!(
                Row,
                r#"
                with photo_stats as (
                  -- Pre-aggregated per owner: joining photos AND follows
                  -- directly onto users fans out to photos × follows rows
                  -- and multiplies the integration sum by follower count.
                  select
                    owner_id,
                    count(*)::bigint as frame_count,
                    coalesce(sum(coalesce(integration_s, exposure_s * coalesce(sessions, 1))), 0)::bigint as integration_seconds
                  from photos
                  where published_at is not null and status = 'ready'
                  group by owner_id
                ),
                stats as (
                  select
                    u.id,
                    u.handle::text as handle,
                    u.display_name,
                    u.created_at,
                    u.cover_photo_id,
                    coalesce(ps.frame_count, 0)::bigint as frame_count,
                    coalesce(ps.integration_seconds, 0)::bigint as integration_seconds,
                    coalesce(count(distinct f.follower_id), 0)::bigint as follower_count
                  from users u
                  left join photo_stats ps on ps.owner_id = u.id
                  left join follows f on f.followed_id = u.id
                  group by u.id, ps.frame_count, ps.integration_seconds
                )
                select
                  id as "id!",
                  handle as "handle!",
                  display_name as "display_name!",
                  created_at as "created_at!",
                  cover_photo_id,
                  frame_count as "frame_count!",
                  integration_seconds as "integration_seconds!",
                  follower_count as "follower_count!"
                  from stats
                 where frame_count > 0
                   and ($1::timestamptz is null or created_at < $1
                        or (created_at = $1 and id > $2::uuid))
                 order by created_at desc, id asc
                 limit $3
                "#,
                cur.as_ref().map(|c| c.created_at),
                cur.as_ref().map(|c| c.id),
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        // "active" (default) — by frame_count
        _ => {
            let cur = q.cursor.as_deref().map(decode::<CountCursor>).transpose()?;
            if cur.as_ref().is_some_and(|c| c.sort != "active") {
                return Err(AppError::bad_request("cursor_invalid"));
            }
            sqlx::query_as!(
                Row,
                r#"
                with photo_stats as (
                  -- Pre-aggregated per owner: joining photos AND follows
                  -- directly onto users fans out to photos × follows rows
                  -- and multiplies the integration sum by follower count.
                  select
                    owner_id,
                    count(*)::bigint as frame_count,
                    coalesce(sum(coalesce(integration_s, exposure_s * coalesce(sessions, 1))), 0)::bigint as integration_seconds
                  from photos
                  where published_at is not null and status = 'ready'
                  group by owner_id
                ),
                stats as (
                  select
                    u.id,
                    u.handle::text as handle,
                    u.display_name,
                    u.created_at,
                    u.cover_photo_id,
                    coalesce(ps.frame_count, 0)::bigint as frame_count,
                    coalesce(ps.integration_seconds, 0)::bigint as integration_seconds,
                    coalesce(count(distinct f.follower_id), 0)::bigint as follower_count
                  from users u
                  left join photo_stats ps on ps.owner_id = u.id
                  left join follows f on f.followed_id = u.id
                  group by u.id, ps.frame_count, ps.integration_seconds
                )
                select
                  id as "id!",
                  handle as "handle!",
                  display_name as "display_name!",
                  created_at as "created_at!",
                  cover_photo_id,
                  frame_count as "frame_count!",
                  integration_seconds as "integration_seconds!",
                  follower_count as "follower_count!"
                  from stats
                 where frame_count > 0
                   and ($1::bigint is null or frame_count < $1
                        or (frame_count = $1 and id > $2::uuid))
                 order by frame_count desc, id asc
                 limit $3
                "#,
                cur.as_ref().map(|c| c.count),
                cur.as_ref().map(|c| c.id),
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    // limit+1 sentinel (same pattern as explore): a final page of
    // exactly `limit` rows used to emit a cursor whose next page was
    // empty — a dead "Load more" for the client.
    let more = rows.len() as i64 > limit;
    let take: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if more {
        take.last().map(|r| match sort {
            "followers" => encode(&CountCursor {
                sort: sort.to_string(),
                count: r.follower_count,
                id: r.id,
            }),
            "recent" => encode(&DateCursor {
                sort: sort.to_string(),
                created_at: r.created_at,
                id: r.id,
            }),
            _ => encode(&CountCursor {
                sort: sort.to_string(),
                count: r.frame_count,
                id: r.id,
            }),
        })
    } else {
        None
    };

    Ok(Json(PhotographerIndexPage {
        items: take.into_iter().map(Into::into).collect(),
        next_cursor,
    }))
}
