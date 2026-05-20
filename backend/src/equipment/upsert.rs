//! equipment_items upsert. Called whenever a photo's equipment fields
//! are written via the verify-form save path.
//!
//! Two-step semantics:
//!
//! 1. `upsert(pool, kind, freetext, submitted_by)` — INSERT … ON CONFLICT
//!    DO NOTHING. Records the first submitter on miss; never overwrites
//!    `submitted_by` on hit (the original discoverer keeps the credit).
//!    Does NOT mutate `usage_count` directly — re-saving the same photo
//!    with the same camera string would otherwise double-count it.
//!
//! 2. `recompute_usage(pool, item_id)` (or `_tx` variant inside a
//!    transaction) — derive `usage_count` from the actual references:
//!    distinct photos via freetext columns, the `photo_filters` junction,
//!    or attached setups whose `setup_items` include the item. Called
//!    after every writer that could change a count so the number
//!    reflects reality instead of a write counter that drifts.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::equipment::normalize_canonical;
use crate::error::AppError;

/// Insert a catalog row for `freetext` if missing. Empty/whitespace input
/// is a no-op (returns Ok). The `submitted_by` user is recorded on miss
/// and left alone on hit — first-discoverer wins.
///
/// `usage_count` is NOT touched here. Call [`recompute_usage`] after the
/// caller's write transaction commits so the count reflects the new set
/// of references.
pub async fn upsert(
    pool: &PgPool,
    kind: &str,
    freetext: &str,
    submitted_by: Uuid,
) -> Result<(), AppError> {
    let display = freetext.trim();
    if display.is_empty() {
        return Ok(());
    }
    let canonical = normalize_canonical(display);
    sqlx::query!(
        r#"
        insert into equipment_items
            (kind, canonical_name, display_name, usage_count, submitted_by, approved_at)
            values ($1, $2, $3, 0, $4, now())
        on conflict (kind, canonical_name) do nothing
        "#,
        kind,
        canonical,
        display,
        submitted_by,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Recompute `usage_count` for `item_id` from the actual photo references.
///
/// The count is the number of distinct photos that touch the item, summed
/// across three independent paths:
///
///   - freetext columns on `photos` whose value equals the item's
///     `display_name` and matches the item's kind (camera/scope/mount/
///     focal_modifier/guiding);
///   - the structured `photo_filters` junction (filter chips);
///   - attached `equipment_setups` that include the item via `setup_items`.
///
/// A photo that mentions an item *both* via freetext AND via a setup is
/// counted in both branches because the three SELECTs are independent —
/// this overcounts by design; deduplication across paths would require a
/// UNION of photo ids and a larger plan. At our scale (small catalog,
/// few rows per item) the simpler arithmetic is fine and predictable.
///
/// O(rows referencing this item), not O(photos in DB). Safe to call
/// per-item in a tight loop after a write.
pub async fn recompute_usage(pool: &PgPool, item_id: Uuid) -> Result<(), AppError> {
    // SAFETY: SQL kept inline because `sqlx::query!` requires a string
    // literal at the call site. The same statement is duplicated in
    // `recompute_usage_tx` below; any edit must be applied to both.
    sqlx::query!(
        r#"
        update equipment_items ei
           set usage_count = (
                 select count(distinct p.id) from photos p
                  where (p.camera         = ei.display_name and ei.kind = 'camera')
                     or (p.scope          = ei.display_name and ei.kind = 'telescope')
                     or (p.mount          = ei.display_name and ei.kind = 'mount')
                     or (p.focal_modifier = ei.display_name and ei.kind = 'focal_modifier')
                     or (p.guiding        = ei.display_name and ei.kind = 'guiding')
               ) + (
                 select count(distinct pf.photo_id) from photo_filters pf
                  where pf.item_id = ei.id
               ) + (
                 select count(distinct p.id) from photos p
                    join setup_items si on si.setup_id = p.setup_id
                   where si.item_id = ei.id
               )
         where ei.id = $1
        "#,
        item_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Same as [`recompute_usage`] but runs inside an open transaction. Use
/// from in-transaction callers (e.g. apply-setup) so the recompute is
/// committed atomically with the write that triggered it.
pub async fn recompute_usage_tx(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        update equipment_items ei
           set usage_count = (
                 select count(distinct p.id) from photos p
                  where (p.camera         = ei.display_name and ei.kind = 'camera')
                     or (p.scope          = ei.display_name and ei.kind = 'telescope')
                     or (p.mount          = ei.display_name and ei.kind = 'mount')
                     or (p.focal_modifier = ei.display_name and ei.kind = 'focal_modifier')
                     or (p.guiding        = ei.display_name and ei.kind = 'guiding')
               ) + (
                 select count(distinct pf.photo_id) from photo_filters pf
                  where pf.item_id = ei.id
               ) + (
                 select count(distinct p.id) from photos p
                    join setup_items si on si.setup_id = p.setup_id
                   where si.item_id = ei.id
               )
         where ei.id = $1
        "#,
        item_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Resolve catalog ids for the (kind, display_name) pairs that the
/// verify-form caller just wrote, then recompute their usage_count.
/// Silently skips kinds whose row no longer exists (e.g. user cleared
/// the field — nothing to recompute against).
pub async fn recompute_usage_for_freetext(
    pool: &PgPool,
    pairs: &[(&str, &str)],
) -> Result<(), AppError> {
    for (kind, freetext) in pairs {
        let trimmed = freetext.trim();
        if trimmed.is_empty() {
            continue;
        }
        let canonical = normalize_canonical(trimmed);
        let id = sqlx::query_scalar!(
            "select id from equipment_items where kind = $1 and canonical_name = $2",
            kind,
            canonical
        )
        .fetch_optional(pool)
        .await?;
        if let Some(id) = id {
            recompute_usage(pool, id).await?;
        }
    }
    Ok(())
}
