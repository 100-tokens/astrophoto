//! Manual target attachment to a photo. Looks up by slug or alias;
//! writes a photo_targets row with source='manual' and is_primary=true
//! when the user picked one explicitly.

use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use sqlx::Postgres;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{PatchTargetsItem, PatchTargetsResponse};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

/// Resolves a freetext target string (slug / alias / canonical name) and inserts
/// or updates a single `source='manual', is_primary=true` row.
///
/// Lenient: returns `Ok(())` silently if the text does not match any target.
/// This preserves `photos.target` (the raw text field) without creating a broken join.
pub async fn attach_primary_by_freetext(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    photo_id: Uuid,
    freetext: &str,
) -> Result<(), AppError> {
    let trimmed = freetext.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Try slug exact, then alias inclusion.
    let target_id: Option<Uuid> = sqlx::query_scalar!(
        r#"
        select id from targets
         where slug = lower($1)
            or $1 = any (aliases)
            or canonical_name ilike $1
         limit 1
        "#,
        trimmed
    )
    .fetch_optional(&mut **tx)
    .await?;

    let Some(tid) = target_id else {
        return Ok(()); // unknown target, just keep photos.target
    };

    sqlx::query!(
        "insert into photo_targets (photo_id, target_id, source, is_primary) \
         values ($1, $2, 'manual', true) \
         on conflict (photo_id, target_id) do update set is_primary = true, source = 'manual'",
        photo_id,
        tid
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Atomically replaces all `source='manual'` rows for `photo_id` with the given slugs.
///
/// - `slugs[0]` gets `is_primary=true`; the rest get `is_primary=false`.
/// - Strictly validates every slug exists (returns 400 with the offending slug otherwise).
/// - Rejects duplicates (400) and lists longer than 5 (400).
/// - Preserves any `source='plate_solve'` rows untouched.
/// - The caller must commit (or roll back) the transaction.
///
/// Note: if a slug already has a `source='plate_solve'` row for this photo, the ON CONFLICT
/// clause promotes it to `source='manual'`. This is intentional — an explicit user selection
/// overrides an automatic result.
pub async fn multi_attach(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    photo_id: Uuid,
    slugs: &[String],
) -> Result<(), AppError> {
    if slugs.len() > 5 {
        return Err(AppError::bad_request("at most 5 targets per photo"));
    }

    let mut seen = std::collections::HashSet::new();
    for s in slugs {
        if !seen.insert(s.as_str()) {
            return Err(AppError::bad_request(format!(
                "duplicate target slug: {s}"
            )));
        }
    }

    // Resolve every slug to a target_id; fail fast on any unknown slug.
    let mut target_ids: Vec<Uuid> = Vec::with_capacity(slugs.len());
    for slug in slugs {
        let id: Option<Uuid> = sqlx::query_scalar!(
            "select id from targets where slug = $1",
            slug
        )
        .fetch_optional(&mut **tx)
        .await?;
        match id {
            Some(id) => target_ids.push(id),
            None => {
                return Err(AppError::bad_request(format!(
                    "unknown target slug: {slug}"
                )));
            }
        }
    }

    // Delete only manual rows; plate_solve rows survive.
    sqlx::query!(
        "delete from photo_targets where photo_id = $1 and source = 'manual'",
        photo_id
    )
    .execute(&mut **tx)
    .await?;

    for (i, tid) in target_ids.iter().enumerate() {
        let is_primary = i == 0;
        sqlx::query!(
            "insert into photo_targets (photo_id, target_id, source, is_primary) \
             values ($1, $2, 'manual', $3) \
             on conflict (photo_id, target_id) do update set source = 'manual', is_primary = $3",
            photo_id,
            tid,
            is_primary
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PatchTargetsBody {
    pub targets: Vec<String>,
}

pub async fn patch_targets(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
    Json(body): Json<PatchTargetsBody>,
) -> Result<Json<PatchTargetsResponse>, AppError> {
    let owner: Option<Uuid> = sqlx::query_scalar!(
        "select owner_id from photos where id = $1",
        photo_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match owner {
        Some(o) if o == user.id => {}
        Some(_) => return Err(AppError::Forbidden),
        None => return Err(AppError::not_found("photo")),
    }

    let mut tx = state.pool.begin().await?;
    multi_attach(&mut tx, photo_id, &body.targets).await?;

    let rows = sqlx::query!(
        r#"
        select t.slug, t.canonical_name, pt.is_primary
        from photo_targets pt
        join targets t on t.id = pt.target_id
        where pt.photo_id = $1 and pt.source = 'manual'
        order by pt.is_primary desc, t.slug
        "#,
        photo_id
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(PatchTargetsResponse {
        targets: rows
            .into_iter()
            .map(|r| PatchTargetsItem {
                slug: r.slug,
                canonical_name: r.canonical_name,
                is_primary: r.is_primary,
            })
            .collect(),
    }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::sync::Arc;

    use sqlx::PgPool;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres as PgImage;
    use uuid::Uuid;

    use super::{AppError, multi_attach};

    /// Spin up a fresh Postgres container with all migrations applied.
    async fn fresh_pool() -> (PgPool, Arc<testcontainers::ContainerAsync<PgImage>>) {
        let pg = PgImage::default().start().await.unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, Arc::new(pg))
    }

    /// Insert a minimal user + photo row and return both IDs.
    async fn seed_photo(pool: &PgPool) -> (Uuid, Uuid) {
        let user_id: Uuid = sqlx::query_scalar(
            "insert into users (email, display_name, handle, password_hash) \
             values ('t@test.local', 'Tester', 'tester', 'x') returning id",
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let photo_id = Uuid::new_v4();
        sqlx::query(
            "insert into photos \
             (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
             values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', 'ABCD1234', now())",
        )
        .bind(photo_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

        (user_id, photo_id)
    }

    #[tokio::test]
    async fn validates_unknown_slug() {
        let (pool, _pg) = fresh_pool().await;
        let (_, photo_id) = seed_photo(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        let err = multi_attach(
            &mut tx,
            photo_id,
            &["m31".to_string(), "this-does-not-exist".to_string()],
        )
        .await
        .unwrap_err();

        match err {
            AppError::BadRequest(msg) => {
                assert!(
                    msg.contains("this-does-not-exist"),
                    "error message should name the offending slug, got: {msg}"
                );
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn first_slug_marked_primary() {
        let (pool, _pg) = fresh_pool().await;
        let (_, photo_id) = seed_photo(&pool).await;

        // ngc-7000 (North America Nebula) is seeded by migration 0010.
        let mut tx = pool.begin().await.unwrap();
        multi_attach(
            &mut tx,
            photo_id,
            &["m42".to_string(), "ngc-7000".to_string()],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Verify is_primary flags.
        let rows: Vec<(String, bool)> = sqlx::query_as(
            "select t.slug, pt.is_primary \
             from photo_targets pt join targets t on t.id = pt.target_id \
             where pt.photo_id = $1 and pt.source = 'manual' \
             order by pt.is_primary desc",
        )
        .bind(photo_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2, "expected 2 rows, got {rows:?}");
        let m42 = rows.iter().find(|(slug, _)| slug == "m42").expect("m42 missing");
        let ngc = rows
            .iter()
            .find(|(slug, _)| slug == "ngc-7000")
            .expect("ngc-7000 missing");
        assert!(m42.1, "m42 should be primary");
        assert!(!ngc.1, "ngc-7000 should not be primary");
    }

    #[tokio::test]
    async fn preserves_plate_solve_rows() {
        let (pool, _pg) = fresh_pool().await;
        let (_, photo_id) = seed_photo(&pool).await;

        // Pre-insert a plate_solve row for ic-434 (seeded by migration 0010).
        let ic434_id: Uuid =
            sqlx::query_scalar("select id from targets where slug = 'ic-434'")
                .fetch_one(&pool)
                .await
                .unwrap();
        sqlx::query(
            "insert into photo_targets (photo_id, target_id, source, is_primary) \
             values ($1, $2, 'plate_solve', false)",
        )
        .bind(photo_id)
        .bind(ic434_id)
        .execute(&pool)
        .await
        .unwrap();

        // multi_attach with only m42.
        let mut tx = pool.begin().await.unwrap();
        multi_attach(&mut tx, photo_id, &["m42".to_string()])
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // ic-434 plate_solve row must still exist.
        let plate_solve_count: i64 = sqlx::query_scalar(
            "select count(*) from photo_targets \
             where photo_id = $1 and target_id = $2 and source = 'plate_solve'",
        )
        .bind(photo_id)
        .bind(ic434_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(plate_solve_count, 1, "plate_solve row for ic-434 must be preserved");

        // m42 manual row must exist with is_primary=true.
        let manual_count: i64 = sqlx::query_scalar(
            "select count(*) from photo_targets pt \
             join targets t on t.id = pt.target_id \
             where pt.photo_id = $1 and t.slug = 'm42' \
               and pt.source = 'manual' and pt.is_primary = true",
        )
        .bind(photo_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(manual_count, 1, "m42 manual+primary row must exist");
    }

    #[tokio::test]
    async fn rejects_more_than_5() {
        let (pool, _pg) = fresh_pool().await;
        let (_, photo_id) = seed_photo(&pool).await;

        let slugs: Vec<String> = (1..=6).map(|i| format!("m{i}")).collect();
        let mut tx = pool.begin().await.unwrap();
        let err = multi_attach(&mut tx, photo_id, &slugs).await.unwrap_err();

        assert!(
            matches!(err, AppError::BadRequest(_)),
            "expected BadRequest for >5 slugs, got {err:?}"
        );
    }

    #[tokio::test]
    async fn rejects_duplicates() {
        let (pool, _pg) = fresh_pool().await;
        let (_, photo_id) = seed_photo(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        let err = multi_attach(
            &mut tx,
            photo_id,
            &["m42".to_string(), "m42".to_string()],
        )
        .await
        .unwrap_err();

        match err {
            AppError::BadRequest(msg) => {
                assert!(
                    msg.contains("m42"),
                    "error should name the duplicate slug, got: {msg}"
                );
            }
            other => panic!("expected BadRequest for duplicate, got {other:?}"),
        }
    }
}
