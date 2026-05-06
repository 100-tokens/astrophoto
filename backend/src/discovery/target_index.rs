//! GET /api/targets — paginated catalog index with search + filters.

use axum::{
    Json,
    extract::{Query, State},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{TargetIndexPage, TargetListItem, TargetPreviewThumb};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct ListQ {
    pub q: Option<String>,
    pub object_type: Option<String>,
    pub constellation: Option<String>,
    pub sort: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

#[derive(serde::Serialize, serde::Deserialize)]
struct PopularCursor {
    count: i64,
    id: Uuid,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NameCursor {
    name: String,
    id: Uuid,
}

fn encode_popular(c: &PopularCursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_popular(s: &str) -> Option<PopularCursor> {
    let b = URL_SAFE_NO_PAD.decode(s).ok()?;
    serde_json::from_slice(&b).ok()
}

fn encode_name(c: &NameCursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_name(s: &str) -> Option<NameCursor> {
    let b = URL_SAFE_NO_PAD.decode(s).ok()?;
    serde_json::from_slice(&b).ok()
}

struct PageRow {
    id: Uuid,
    slug: String,
    canonical_name: String,
    object_type: Option<String>,
    constellation: Option<String>,
    magnitude_v: Option<f32>,
    photo_count: i64,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<TargetIndexPage>, AppError> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("popular");

    let q_str = q.q.as_deref();
    let obj = q.object_type.as_deref();
    let cons = q.constellation.as_deref();

    let rows: Vec<PageRow> = match sort {
        "name" => {
            let cur = q.cursor.as_deref().and_then(decode_name);
            let cur_name = cur.as_ref().map(|c| c.name.clone());
            let cur_id = cur.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PageRow,
                r#"
                select t.id as "id!", t.slug as "slug!", t.canonical_name as "canonical_name!",
                       t.object_type, t.constellation, t.magnitude_v,
                       coalesce((select count(*) from photo_targets pt
                                 join photos p on p.id = pt.photo_id
                                 where pt.target_id = t.id
                                   and p.published_at is not null
                                   and p.status = 'ready'), 0)::int8 as "photo_count!"
                from targets t
                where ($1::text is null or
                       t.canonical_name ilike '%' || $1 || '%' or
                       t.slug ilike '%' || $1 || '%' or
                       exists (select 1 from unnest(t.aliases) a where a ilike '%' || $1 || '%'))
                  and ($2::text is null or t.object_type = $2)
                  and ($3::text is null or t.constellation = $3)
                  and ($4::text is null or (t.canonical_name, t.id) > ($4, $5))
                order by t.canonical_name asc, t.id asc
                limit $6
                "#,
                q_str,
                obj,
                cons,
                cur_name,
                cur_id,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        _ /* popular */ => {
            let cur = q.cursor.as_deref().and_then(decode_popular);
            let cur_count = cur.as_ref().map(|c| c.count);
            let cur_id = cur.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PageRow,
                r#"
                with t_with_counts as (
                  select t.id, t.slug, t.canonical_name, t.object_type, t.constellation, t.magnitude_v,
                         coalesce((select count(*) from photo_targets pt
                                   join photos p on p.id = pt.photo_id
                                   where pt.target_id = t.id
                                     and p.published_at is not null
                                     and p.status = 'ready'), 0)::int8 as photo_count
                    from targets t
                   where ($1::text is null or
                          t.canonical_name ilike '%' || $1 || '%' or
                          t.slug ilike '%' || $1 || '%' or
                          exists (select 1 from unnest(t.aliases) a where a ilike '%' || $1 || '%'))
                     and ($2::text is null or t.object_type = $2)
                     and ($3::text is null or t.constellation = $3)
                )
                select id as "id!", slug as "slug!", canonical_name as "canonical_name!",
                       object_type, constellation, magnitude_v, photo_count as "photo_count!"
                  from t_with_counts
                 where ($4::int8 is null or photo_count < $4 or (photo_count = $4 and id < $5))
                 order by photo_count desc, id desc
                 limit $6
                "#,
                q_str,
                obj,
                cons,
                cur_count,
                cur_id,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    let more = rows.len() as i64 > limit;
    let kept: Vec<PageRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if !more {
        None
    } else {
        kept.last().map(|last| match sort {
            "name" => encode_name(&NameCursor {
                name: last.canonical_name.clone(),
                id: last.id,
            }),
            _ => encode_popular(&PopularCursor {
                count: last.photo_count,
                id: last.id,
            }),
        })
    };

    let target_ids: Vec<Uuid> = kept.iter().map(|r| r.id).collect();
    let thumb_rows = sqlx::query!(
        r#"
        select t.id as "target_id!", p.short_id as "short_id!", p.blurhash
          from unnest($1::uuid[]) as t(id)
          join lateral (
            select p.short_id, p.blurhash, p.appreciations_count, p.published_at
              from photo_targets pt
              join photos p on p.id = pt.photo_id
             where pt.target_id = t.id
               and p.published_at is not null
               and p.status = 'ready'
             order by p.appreciations_count desc, p.published_at desc
             limit 3
          ) p on true
        "#,
        &target_ids
    )
    .fetch_all(&state.pool)
    .await?;

    let mut by_target: std::collections::HashMap<Uuid, Vec<TargetPreviewThumb>> =
        std::collections::HashMap::new();
    for tr in thumb_rows {
        by_target
            .entry(tr.target_id)
            .or_default()
            .push(TargetPreviewThumb {
                short_id: tr.short_id,
                blurhash: tr.blurhash,
            });
    }

    Ok(Json(TargetIndexPage {
        targets: kept
            .into_iter()
            .map(|r| TargetListItem {
                slug: r.slug,
                canonical_name: r.canonical_name,
                object_type: r.object_type,
                constellation: r.constellation,
                magnitude_v: r.magnitude_v,
                photo_count: r.photo_count,
                preview_thumbs: by_target.remove(&r.id).unwrap_or_default(),
            })
            .collect(),
        next_cursor,
    }))
}

#[cfg(test)]
mod cursor_tests {
    use super::*;

    #[test]
    fn popular_cursor_roundtrip() {
        let c = PopularCursor {
            count: 12,
            id: Uuid::new_v4(),
        };
        let encoded = encode_popular(&c);
        let decoded = decode_popular(&encoded).unwrap();
        assert_eq!(decoded.count, c.count);
        assert_eq!(decoded.id, c.id);
    }

    #[test]
    fn name_cursor_roundtrip() {
        let c = NameCursor {
            name: "Andromeda Galaxy".into(),
            id: Uuid::new_v4(),
        };
        let encoded = encode_name(&c);
        let decoded = decode_name(&encoded).unwrap();
        assert_eq!(decoded.name, c.name);
        assert_eq!(decoded.id, c.id);
    }

    #[test]
    fn invalid_cursor_returns_none() {
        assert!(decode_popular("not-base64!!!").is_none());
        assert!(decode_name("also-not!!!").is_none());
    }
}
