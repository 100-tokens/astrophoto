use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{CategoryPage, DiscoveryPage, DiscoveryPhoto};
use crate::discovery::cursor::{self, Cursor};
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

const CATEGORIES: &[&str] = &[
    "dso",
    "planetary",
    "lunar",
    "solar",
    "wide_field",
    "nightscape",
    "other",
];

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub sort: Option<String>,
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
    Path(cat): Path<String>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    // Accept URL-friendly hyphenated form ("wide-field") in addition to
    // the DB enum form ("wide_field"). The rest of the site uses hyphens
    // for slugs (`/t/ngc-7000`, `/tag/foo-bar`), so the category page
    // should too — internally we normalise to underscores.
    let cat = cat.replace('-', "_");
    if !CATEGORIES.contains(&cat.as_str()) {
        return Err(AppError::not_found("category"));
    }

    // Count total ready+published photos in this category.
    let photo_count: i64 = sqlx::query_scalar!(
        r#"
        select count(*)::int8 as "count!"
        from photos
        where category = $1
          and published_at is not null
          and status = 'ready'
          and not exists (select 1 from users du where du.id = photos.owner_id and du.pending_deletion_at is not null)
        "#,
        cat
    )
    .fetch_one(&state.pool)
    .await?;

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
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
                where p.category = $1
                  and p.published_at is not null
                  and p.status = 'ready'
                  and u.pending_deletion_at is null
                  and ($2::int4 is null or
                       p.appreciations_count < $2 or
                       (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                order by p.appreciations_count desc, p.published_at desc, p.id desc
                limit $5
                "#,
                cat,
                cur_apps,
                cur_pub,
                cur_id,
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
                where p.category = $1
                  and p.published_at is not null
                  and p.status = 'ready'
                  and u.pending_deletion_at is null
                  and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                order by p.published_at desc, p.id desc
                limit $4
                "#,
                cat,
                cur_pub,
                cur_id,
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

    Ok(Json(CategoryPage {
        category: cat,
        photo_count,
        page: DiscoveryPage {
            photos: take
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
            next_cursor,
        },
    }))
}
