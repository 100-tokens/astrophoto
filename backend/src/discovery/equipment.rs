use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{
    DiscoveryPage, DiscoveryPhoto, EquipmentMeta, EquipmentPage, EquipmentPaired, EquipmentSibling,
};
use crate::discovery::cursor::{self, Cursor};
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

/// Whitelist of accepted kind values.
const VALID_KINDS: &[&str] = &["telescope", "camera", "mount", "filter", "guiding"];

/// Equipment items have `canonical_name` like "sony a7iv" (lowercase with
/// spaces). URLs need a hyphenated slug. These helpers convert between the
/// two — the URL form is `slug_for(canonical)` and the DB lookup needs
/// `canonical_for(slug)` to round-trip.
fn slug_for(canonical: &str) -> String {
    canonical.replace(' ', "-")
}
fn canonical_for(slug: &str) -> String {
    slug.replace('-', " ")
}

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub sort: Option<String>, // "newest" (default) | "most-appreciated"
    pub category: Option<String>,
}

struct PairedRow {
    kind: String,
    slug: String,
    display_name: String,
    shared_count: i64,
}

struct SiblingRow {
    kind: String,
    canonical_name: String,
    display_name: String,
    usage_count: i32,
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
    Path((kind, slug)): Path<(String, String)>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    // Validate kind — any other value is a "page doesn't exist" 404.
    if !VALID_KINDS.contains(&kind.as_str()) {
        return Err(AppError::not_found("equipment_kind"));
    }

    // URL slug ("sony-a7iv") → canonical ("sony a7iv") for every DB lookup
    // below. Shadow `slug` so the rest of the handler doesn't accidentally
    // use the hyphenated URL form against `canonical_name` columns.
    let slug = canonical_for(&slug);
    let canonical = slug.clone();
    // Catalog v2: a brand like "Sky-Watcher" lands in `canonical_name`
    // as `"sky-watcher esprit 100"` (one hyphen + spaces). The browse
    // page builds its URL by collapsing both into `-`, so the slug
    // arrives here as `"sky-watcher-esprit-100"`; the naive
    // `canonical_for` (replace '-' with ' ') turns that back into
    // `"sky watcher esprit 100"` which doesn't equal the DB row.
    // Match against both forms so hyphenated brands work without
    // a separate slug column. The non-hyphen case stays a fast
    // index lookup; the second predicate is the fallback when the
    // first misses.
    let item = sqlx::query!(
        r#"
        select id as "id!", kind as "kind!", canonical_name as "canonical_name!",
               display_name as "display_name!", usage_count as "usage_count!"
        from equipment_items
        where kind = $1
          and (canonical_name = $2 or replace(canonical_name, '-', ' ') = $2)
        limit 1
        "#,
        kind,
        canonical
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(item) = item else {
        return Err(AppError::not_found("equipment"));
    };

    // Count photos that use this equipment item (across both legacy and — if present — modern field).
    // The photos table has: camera (0001), scope/mount/filters/guiding (0009).
    // There are no photos.equipment_* columns — only users.equipment_* which we don't use here.
    let photo_count: i64 = match kind.as_str() {
        "telescope" => {
            sqlx::query_scalar!(
                r#"select count(*)::int8 as "c!" from photos
               where published_at is not null and status = 'ready'
               and lower(scope) = $1"#,
                slug
            )
            .fetch_one(&state.pool)
            .await?
        }
        "camera" => {
            sqlx::query_scalar!(
                r#"select count(*)::int8 as "c!" from photos
               where published_at is not null and status = 'ready'
               and lower(camera) = $1"#,
                slug
            )
            .fetch_one(&state.pool)
            .await?
        }
        "mount" => {
            sqlx::query_scalar!(
                r#"select count(*)::int8 as "c!" from photos
               where published_at is not null and status = 'ready'
               and lower(mount) = $1"#,
                slug
            )
            .fetch_one(&state.pool)
            .await?
        }
        "filter" => {
            sqlx::query_scalar!(
                r#"select count(*)::int8 as "c!" from photos
               where published_at is not null and status = 'ready'
               and lower(filters) = $1"#,
                slug
            )
            .fetch_one(&state.pool)
            .await?
        }
        "guiding" => {
            sqlx::query_scalar!(
                r#"select count(*)::int8 as "c!" from photos
               where published_at is not null and status = 'ready'
               and lower(guiding) = $1"#,
                slug
            )
            .fetch_one(&state.pool)
            .await?
        }
        _ => unreachable!("kind already validated above"),
    };

    // Paginated photos query — per-kind dispatch for compile-time column checking.
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
    let category = q.category.as_deref();
    let cursor = q.cursor.as_deref().map(cursor::decode).transpose()?;
    let cur_pub = cursor.as_ref().map(|c| c.published_at);
    let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
    let cur_apps = cursor.as_ref().and_then(|c| c.appreciations);

    let rows: Vec<Row> = match (kind.as_str(), sort) {
        ("telescope", "most-appreciated") => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.scope) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::int4 is null or p.appreciations_count < $2 or
                      (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                 and ($5::text is null or p.category = $5)
               order by p.appreciations_count desc, p.published_at desc, p.id desc
               limit $6"#,
                slug,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("telescope", _) => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.scope) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                 and ($4::text is null or p.category = $4)
               order by p.published_at desc, p.id desc
               limit $5"#,
                slug,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("camera", "most-appreciated") => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.camera) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::int4 is null or p.appreciations_count < $2 or
                      (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                 and ($5::text is null or p.category = $5)
               order by p.appreciations_count desc, p.published_at desc, p.id desc
               limit $6"#,
                slug,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("camera", _) => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.camera) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                 and ($4::text is null or p.category = $4)
               order by p.published_at desc, p.id desc
               limit $5"#,
                slug,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("mount", "most-appreciated") => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.mount) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::int4 is null or p.appreciations_count < $2 or
                      (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                 and ($5::text is null or p.category = $5)
               order by p.appreciations_count desc, p.published_at desc, p.id desc
               limit $6"#,
                slug,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("mount", _) => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.mount) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                 and ($4::text is null or p.category = $4)
               order by p.published_at desc, p.id desc
               limit $5"#,
                slug,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("filter", "most-appreciated") => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.filters) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::int4 is null or p.appreciations_count < $2 or
                      (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                 and ($5::text is null or p.category = $5)
               order by p.appreciations_count desc, p.published_at desc, p.id desc
               limit $6"#,
                slug,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("filter", _) => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.filters) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                 and ($4::text is null or p.category = $4)
               order by p.published_at desc, p.id desc
               limit $5"#,
                slug,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("guiding", "most-appreciated") => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.guiding) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::int4 is null or p.appreciations_count < $2 or
                      (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
                 and ($5::text is null or p.category = $5)
               order by p.appreciations_count desc, p.published_at desc, p.id desc
               limit $6"#,
                slug,
                cur_apps,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        ("guiding", _) => {
            sqlx::query_as!(
                Row,
                r#"select p.id as "id!", p.short_id as "short_id!", p.target,
                      p.width, p.height, p.blurhash,
                      p.appreciations_count as "appreciations_count!",
                      p.published_at,
                      u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
               from photos p join users u on u.id = p.owner_id
               where lower(p.guiding) = $1
                 and p.published_at is not null and p.status = 'ready'
                 and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
                 and ($4::text is null or p.category = $4)
               order by p.published_at desc, p.id desc
               limit $5"#,
                slug,
                cur_pub,
                cur_id,
                category,
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        _ => unreachable!("kind already validated above"),
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

    // "Often paired with" rail: top 4 other equipment items that co-occur
    // most on photos that use THIS item.
    // We join equipment_items to photos via the kind→column mapping.
    // The photos that "use this item" are the same set as the photos query above,
    // but we fetch them here via a subquery for the join.
    let paired_rows = sqlx::query_as!(
        PairedRow,
        r#"
        select
            ei.kind           as "kind!",
            ei.canonical_name as "slug!",
            ei.display_name   as "display_name!",
            count(*)::int8    as "shared_count!"
        from equipment_items ei
        join photos p on (
            (ei.kind = 'telescope' and lower(p.scope)   = ei.canonical_name) or
            (ei.kind = 'camera'    and lower(p.camera)  = ei.canonical_name) or
            (ei.kind = 'mount'     and lower(p.mount)   = ei.canonical_name) or
            (ei.kind = 'filter'    and lower(p.filters) = ei.canonical_name) or
            (ei.kind = 'guiding'   and lower(p.guiding) = ei.canonical_name)
        )
        where p.published_at is not null and p.status = 'ready'
          and not (ei.kind = $1 and ei.canonical_name = $2)
          and exists (
              select 1 from photos p2
              where p2.id = p.id
                and (
                    ($1 = 'telescope' and lower(p2.scope)   = $2) or
                    ($1 = 'camera'    and lower(p2.camera)  = $2) or
                    ($1 = 'mount'     and lower(p2.mount)   = $2) or
                    ($1 = 'filter'    and lower(p2.filters) = $2) or
                    ($1 = 'guiding'   and lower(p2.guiding) = $2)
                )
          )
        group by ei.kind, ei.canonical_name, ei.display_name
        order by count(*) desc
        limit 4
        "#,
        kind,
        slug
    )
    .fetch_all(&state.pool)
    .await?;

    // "Other <brand>" sibling rail: same-kind catalog items whose
    // canonical_name shares the leading brand token. Names without a
    // space (single-token names) have no brand prefix → no siblings.
    // Sort by usage_count desc so the most-used variants surface first.
    let brand_prefix: Option<String> = item
        .canonical_name
        .split_whitespace()
        .next()
        .map(|t| t.to_string())
        .filter(|_| item.canonical_name.contains(' '));

    let siblings_rows: Vec<SiblingRow> = if let Some(ref brand) = brand_prefix {
        // Escape '%' and '_' so a brand name containing them doesn't widen
        // the match. Then concat with " %" to enforce the leading token
        // boundary (matches "<brand> <rest>" only).
        let escaped = brand
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("{} %", escaped);
        sqlx::query_as!(
            SiblingRow,
            r#"
            select
                kind            as "kind!",
                canonical_name  as "canonical_name!",
                display_name    as "display_name!",
                usage_count     as "usage_count!"
            from equipment_items
            where kind = $1
              and status = 'approved'
              and canonical_name like $2 escape '\'
              and canonical_name <> $3
            order by usage_count desc, display_name asc
            limit 6
            "#,
            item.kind,
            pattern,
            item.canonical_name
        )
        .fetch_all(&state.pool)
        .await?
    } else {
        Vec::new()
    };

    Ok(Json(EquipmentPage {
        equipment: EquipmentMeta {
            id: item.id.to_string(),
            kind: item.kind,
            slug: slug_for(&item.canonical_name),
            canonical_name: item.canonical_name,
            display_name: item.display_name,
            photo_count,
        },
        siblings: siblings_rows
            .into_iter()
            .map(|r| EquipmentSibling {
                kind: r.kind,
                slug: slug_for(&r.canonical_name),
                display_name: r.display_name,
                usage_count: r.usage_count,
            })
            .collect(),
        paired: paired_rows
            .into_iter()
            .map(|r| EquipmentPaired {
                kind: r.kind,
                slug: slug_for(&r.slug),
                display_name: r.display_name,
                shared_count: r.shared_count,
            })
            .collect(),
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
