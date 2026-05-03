use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{DiscoveryPage, DiscoveryPhoto, TagMeta, TagPage};
use crate::discovery::cursor::{self, Cursor};
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub sort: Option<String>,
    pub category: Option<String>,
}

struct Row {
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

pub async fn get(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let tag = sqlx::query!(
        r#"
        select t.id as "id!", t.slug as "slug!", t.name as "name!",
               (select count(*) from photo_tags pt join photos p on p.id = pt.photo_id
                where pt.tag_id = t.id and p.published_at is not null and p.status = 'ready')::int8 as "photo_count!"
        from tags t
        where t.slug = $1
        "#,
        slug
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(t) = tag else {
        return Err(AppError::not_found("tag"));
    };

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
    let category = q.category.as_deref();
    let cursor = q.cursor.as_deref().map(cursor::decode).transpose()?;
    let cur_pub = cursor.as_ref().map(|c| c.published_at);
    let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
    let cur_apps = cursor.as_ref().and_then(|c| c.appreciations);

    let rows: Vec<Row> = match sort {
        "most-appreciated" => {
            sqlx::query_as!(
                Row,
                r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            join photo_tags pt on pt.photo_id = p.id
            where pt.tag_id = $1
              and p.published_at is not null
              and p.status = 'ready'
              and ($2::int4 is null or p.appreciations_count < $2 or
                   (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
              and ($5::text is null or p.category = $5)
            order by p.appreciations_count desc, p.published_at desc, p.id desc
            limit $6
            "#,
                t.id,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        _ => {
            sqlx::query_as!(
                Row,
                r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            join photo_tags pt on pt.photo_id = p.id
            where pt.tag_id = $1
              and p.published_at is not null
              and p.status = 'ready'
              and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
              and ($4::text is null or p.category = $4)
            order by p.published_at desc, p.id desc
            limit $5
            "#,
                t.id,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    let more = rows.len() as i64 > limit;
    let take: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if more {
        take.last().and_then(|last| {
            last.published_at.map(|published_at| {
                cursor::encode(&Cursor {
                    published_at,
                    id: last.id,
                    appreciations: if sort == "most-appreciated" {
                        Some(last.appreciations_count)
                    } else {
                        None
                    },
                })
            })
        })
    } else {
        None
    };

    Ok(Json(TagPage {
        tag: TagMeta {
            slug: t.slug,
            name: t.name,
            photo_count: t.photo_count,
        },
        page: DiscoveryPage {
            photos: take
                .into_iter()
                .map(|r| DiscoveryPhoto {
                    id: r.id,
                    short_id: r.short_id,
                    target: r.target,
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
            next_cursor,
        },
    }))
}
