//! GET /api/targets — paginated catalog index with search + filters.

use axum::{
    Json,
    extract::{Query, State},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Datelike;
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{TargetIndexPage, TargetListItem, TargetPreviewThumb};
use crate::discovery::opposition::circular_doy_distance;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct ListQ {
    pub q: Option<String>,
    /// One or more object types, comma-joined (e.g. `G,Neb,OCl`). Empty = all.
    pub object_type: Option<String>,
    pub constellation: Option<String>,
    pub sort: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    /// When true, only return targets that have at least one published,
    /// ready photo. The public `/t` index defaults this on so the catalog
    /// shows photographed objects, not the ~12k empty OpenNGC stubs.
    /// Autocomplete / search backends omit it to get the full catalog.
    pub has_photos: Option<bool>,
    /// Inclusive lower / exclusive upper bound on the object's major axis
    /// (arcmin) — the focal-length-hinted size buckets on `/t`.
    pub size_min: Option<f32>,
    pub size_max: Option<f32>,
}

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

/// Sentinel for the effective sort key of targets with no opposition date
/// (unknown RA) so they keyset-paginate last instead of via SQL NULL ordering.
const NULLS_LAST: i32 = 9999;

/// "Optimal now" only surfaces objects within this many days of opposition —
/// i.e. observable in a dark sky this season. Beyond it the object is near
/// conjunction (up in the daytime sky), so it isn't an "optimal" target now;
/// without this cut a sparse filter would pad page 1 with un-shootable objects.
/// A distance proxy: it ignores observer latitude (high-dec circumpolar
/// objects stay up out of season), consistent with the RA-only opposition scope.
const OPTIMAL_WINDOW_DAYS: i32 = 90;

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

#[derive(serde::Serialize, serde::Deserialize)]
struct OptimalCursor {
    /// Circular day-of-year distance from "today" to the object's opposition,
    /// or the NULLS_LAST sentinel for targets with no opposition date.
    dist: i32,
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

fn encode_optimal(c: &OptimalCursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_optimal(s: &str) -> Option<OptimalCursor> {
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
    opposition_doy: Option<i16>,
    photo_count: i64,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<TargetIndexPage>, AppError> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("popular");

    let q_str = q.q.as_deref();
    let cons = q.constellation.as_deref();
    let has_photos = q.has_photos.unwrap_or(false);
    let size_min = q.size_min;
    let size_max = q.size_max;

    // Multiple object types arrive comma-joined; an empty/blank list means
    // "no type filter" (bind NULL so the `= any(...)` clause is skipped).
    let types: Option<Vec<String>> = q.object_type.as_deref().and_then(|s| {
        let v: Vec<String> = s
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        (!v.is_empty()).then_some(v)
    });

    // "Optimal now" ranks by circular distance from today's day-of-year to each
    // object's opposition. `ordinal()` is 1..=366; clamp leap-day 366 into the
    // non-leap reference calendar that `opposition_doy` uses.
    let today = (chrono::Utc::now().ordinal() as i32).clamp(1, 365);

    let rows: Vec<PageRow> = match sort {
        "name" => {
            let cur = q.cursor.as_deref().and_then(decode_name);
            let cur_name = cur.as_ref().map(|c| c.name.clone());
            let cur_id = cur.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PageRow,
                r#"
                select t.id as "id!", t.slug as "slug!", t.canonical_name as "canonical_name!",
                       t.object_type, t.constellation, t.magnitude_v, t.opposition_doy,
                       coalesce((select count(*) from photo_targets pt
                                 join photos p on p.id = pt.photo_id
                                 where pt.target_id = t.id
                                   and p.published_at is not null
                                   and p.status = 'ready'
                                   and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)), 0)::int8 as "photo_count!"
                from targets t
                where ($1::text is null or
                       t.canonical_name ilike '%' || $1 || '%' or
                       t.slug ilike '%' || $1 || '%' or
                       t.aliases_text ilike '%' || $1 || '%')
                  and ($2::text[] is null or t.object_type = any($2))
                  and ($3::text is null or t.constellation = $3)
                  and ($4::text is null or (t.canonical_name, t.id) > ($4, $5))
                  and ($7::bool is not true or exists (
                        select 1 from photo_targets pt
                        join photos p on p.id = pt.photo_id
                        where pt.target_id = t.id
                          and p.published_at is not null
                          and p.status = 'ready'
                          and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)))
                  and ($8::real is null or t.major_axis_arcmin >= $8)
                  and ($9::real is null or t.major_axis_arcmin < $9)
                order by t.canonical_name asc, t.id asc
                limit $6
                "#,
                q_str,
                types.as_deref(),
                cons,
                cur_name,
                cur_id,
                limit + 1,
                has_photos,
                size_min,
                size_max
            )
            .fetch_all(&state.pool)
            .await?
        }
        "optimal" => {
            // Best-to-observe-now order: ascending circular day-of-year distance
            // from `today` to each object's opposition (see
            // opposition::circular_doy_distance — the SQL mirrors it exactly).
            // Targets without a known RA (null opposition_doy) sort last via the
            // NULLS_LAST sentinel, keyset-paginated on (distance, id).
            let cur = q.cursor.as_deref().and_then(decode_optimal);
            let cur_dist = cur.as_ref().map(|c| c.dist);
            let cur_id = cur.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
            sqlx::query_as!(
                PageRow,
                r#"
                select t.id as "id!", t.slug as "slug!", t.canonical_name as "canonical_name!",
                       t.object_type, t.constellation, t.magnitude_v, t.opposition_doy,
                       coalesce((select count(*) from photo_targets pt
                                 join photos p on p.id = pt.photo_id
                                 where pt.target_id = t.id
                                   and p.published_at is not null
                                   and p.status = 'ready'
                                   and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)), 0)::int8 as "photo_count!"
                from targets t
                where ($1::text is null or
                       t.canonical_name ilike '%' || $1 || '%' or
                       t.slug ilike '%' || $1 || '%' or
                       t.aliases_text ilike '%' || $1 || '%')
                  and ($2::text[] is null or t.object_type = any($2))
                  and ($3::text is null or t.constellation = $3)
                  and ($4::int4 is null or
                       (coalesce(least(abs(t.opposition_doy::int4 - $8::int4),
                                       365 - abs(t.opposition_doy::int4 - $8::int4)), 9999), t.id)
                        > ($4, $5))
                  and ($7::bool is not true or exists (
                        select 1 from photo_targets pt
                        join photos p on p.id = pt.photo_id
                        where pt.target_id = t.id
                          and p.published_at is not null
                          and p.status = 'ready'
                          and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)))
                  and ($9::real is null or t.major_axis_arcmin >= $9)
                  and ($10::real is null or t.major_axis_arcmin < $10)
                  and least(abs(t.opposition_doy::int4 - $8::int4),
                            365 - abs(t.opposition_doy::int4 - $8::int4)) <= $11::int4
                order by coalesce(least(abs(t.opposition_doy::int4 - $8::int4),
                                        365 - abs(t.opposition_doy::int4 - $8::int4)), 9999) asc,
                         t.id asc
                limit $6
                "#,
                q_str,
                types.as_deref(),
                cons,
                cur_dist,
                cur_id,
                limit + 1,
                has_photos,
                today,
                size_min,
                size_max,
                OPTIMAL_WINDOW_DAYS
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
                         t.opposition_doy,
                         coalesce((select count(*) from photo_targets pt
                                   join photos p on p.id = pt.photo_id
                                   where pt.target_id = t.id
                                     and p.published_at is not null
                                     and p.status = 'ready'
                                     and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)), 0)::int8 as photo_count
                    from targets t
                   where ($1::text is null or
                          t.canonical_name ilike '%' || $1 || '%' or
                          t.slug ilike '%' || $1 || '%' or
                          t.aliases_text ilike '%' || $1 || '%')
                     and ($2::text[] is null or t.object_type = any($2))
                     and ($3::text is null or t.constellation = $3)
                     and ($7::bool is not true or exists (
                           select 1 from photo_targets pt
                           join photos p on p.id = pt.photo_id
                           where pt.target_id = t.id
                             and p.published_at is not null
                             and p.status = 'ready'
                             and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)))
                     and ($8::real is null or t.major_axis_arcmin >= $8)
                     and ($9::real is null or t.major_axis_arcmin < $9)
                )
                select id as "id!", slug as "slug!", canonical_name as "canonical_name!",
                       object_type, constellation, magnitude_v, opposition_doy,
                       photo_count as "photo_count!"
                  from t_with_counts
                 where ($4::int8 is null or photo_count < $4 or (photo_count = $4 and id < $5))
                 order by photo_count desc, id desc
                 limit $6
                "#,
                q_str,
                types.as_deref(),
                cons,
                cur_count,
                cur_id,
                limit + 1,
                has_photos,
                size_min,
                size_max
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
            "optimal" => encode_optimal(&OptimalCursor {
                dist: last
                    .opposition_doy
                    .map(|o| circular_doy_distance(o as i32, today))
                    .unwrap_or(NULLS_LAST),
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
        select t.id as "target_id!", p.id as "photo_id!", p.short_id as "short_id!", p.blurhash
          from unnest($1::uuid[]) as t(id)
          join lateral (
            select p.id, p.short_id, p.blurhash, p.appreciations_count, p.published_at
              from photo_targets pt
              join photos p on p.id = pt.photo_id
             where pt.target_id = t.id
               and p.published_at is not null
               and p.status = 'ready'
               and not exists (select 1 from users du where du.id = p.owner_id and du.pending_deletion_at is not null)
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
                photo_id: tr.photo_id.to_string(),
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
                opposition_doy: r.opposition_doy,
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
    fn optimal_cursor_roundtrip() {
        let c = OptimalCursor {
            dist: 37,
            id: Uuid::new_v4(),
        };
        let encoded = encode_optimal(&c);
        let decoded = decode_optimal(&encoded).unwrap();
        assert_eq!(decoded.dist, c.dist);
        assert_eq!(decoded.id, c.id);
    }

    #[test]
    fn invalid_cursor_returns_none() {
        assert!(decode_popular("not-base64!!!").is_none());
        assert!(decode_name("also-not!!!").is_none());
        assert!(decode_optimal("nope!!!").is_none());
    }
}
