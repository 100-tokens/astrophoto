use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{GalleryPage, GalleryPhoto};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit:  Option<i64>,
    pub sort:   Option<String>, // "newest" (default) | "popular"
}

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

#[derive(serde::Serialize, serde::Deserialize)]
struct Cursor {
    published_at: chrono::DateTime<chrono::Utc>,
    id: Uuid,
    #[serde(default)]
    appreciations: Option<i32>,
}

fn encode_cursor(c: &Cursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_cursor(s: &str) -> Result<Cursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AppError::bad_request("cursor_invalid"))?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::bad_request("cursor_invalid"))
}

struct PhotoRow {
    id: Uuid,
    short_id: String,
    target: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    blurhash: Option<String>,
    appreciations_count: i32,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        "select id from users where handle = $1",
        handle.to_lowercase()
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(u) = user else {
        return Err(AppError::not_found("user"));
    };

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
    let cursor = q.cursor.as_deref().map(decode_cursor).transpose()?;

    let rows: Vec<PhotoRow> = match sort {
        "popular" => {
            let cur_apps = cursor.as_ref().and_then(|c| c.appreciations);
            let cur_pub = cursor.as_ref().map(|c| c.published_at);
            let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PhotoRow,
                r#"
                select id, short_id, target, width, height, blurhash, appreciations_count, published_at
                from photos
                where owner_id = $1
                  and published_at is not null
                  and status = 'ready'
                  and ($2::int4 is null or appreciations_count < $2 or
                       (appreciations_count = $2 and (published_at, id) < ($3, $4)))
                order by appreciations_count desc, published_at desc, id desc
                limit $5
                "#,
                u.id,
                cur_apps,
                cur_pub,
                cur_id,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        _ => {
            let cur_pub = cursor.as_ref().map(|c| c.published_at);
            let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PhotoRow,
                r#"
                select id, short_id, target, width, height, blurhash, appreciations_count, published_at
                from photos
                where owner_id = $1
                  and published_at is not null
                  and status = 'ready'
                  and ($2::timestamptz is null or (published_at, id) < ($2, $3))
                order by published_at desc, id desc
                limit $4
                "#,
                u.id,
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

    let next_cursor = if more && !take.is_empty() {
        let last = take.last().unwrap();
        Some(encode_cursor(&Cursor {
            published_at: last.published_at.unwrap(),
            id: last.id,
            appreciations: if sort == "popular" {
                Some(last.appreciations_count)
            } else {
                None
            },
        }))
    } else {
        None
    };

    Ok(Json(GalleryPage {
        photos: take
            .into_iter()
            .map(|r| GalleryPhoto {
                id: r.id,
                short_id: r.short_id,
                target: r.target,
                width: r.width,
                height: r.height,
                blurhash: r.blurhash,
                appreciations_count: r.appreciations_count,
                published_at: r.published_at,
            })
            .collect(),
        next_cursor,
    }))
}
