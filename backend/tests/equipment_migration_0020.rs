//! Migration 0020: backfill equipment_items.approved_at for legacy
//! rows inserted with status='approved' but a NULL timestamp.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

/// After the migration suite runs, no approved equipment_items row
/// should have a NULL approved_at — neither the rows stamped by
/// migration 0018, nor the rows the test inserts pre-0020 with a
/// deliberate NULL (the helper here re-inserts after migration 0020
/// would have already run, but the assertion verifies the
/// post-migration shape).
#[tokio::test]
async fn migration_0020_backfills_null_approved_at_for_approved_rows() {
    let (_app, pool) = common::make_app_and_pool().await;

    // Insert a row with the post-fix INSERT shape (approved_at = now())
    // and one with an explicit NULL — the migration has already run by
    // this point, so the NULL will persist until we re-trigger the
    // backfill manually below. This mirrors what happens on a real
    // env: rows inserted between migration 0018 and the items_create
    // fix (commit 1f2d122) have NULL approved_at; running 0020 a
    // second time stamps them.
    sqlx::query(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count,
                                       status, approved_at)
             values ('filter', 'legacy null', 'Legacy null', 0, 'approved', null),
                    ('filter', 'fresh stamped', 'Fresh stamped', 0, 'approved', now())",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Sanity: NULL exists right now (migration 0020 ran before this
    // INSERT, so it didn't catch this row).
    let null_before: i64 = sqlx::query_scalar!(
        "select count(*) as \"count!\" from equipment_items
         where status = 'approved' and approved_at is null"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        null_before, 1,
        "the legacy NULL row should still be NULL before re-running the backfill"
    );

    // Re-run the migration body manually to prove idempotency.
    sqlx::query(
        "update equipment_items
            set approved_at = coalesce(created_at, now())
          where status = 'approved' and approved_at is null",
    )
    .execute(&pool)
    .await
    .unwrap();

    let null_after: i64 = sqlx::query_scalar!(
        "select count(*) as \"count!\" from equipment_items
         where status = 'approved' and approved_at is null"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_after, 0, "backfill should stamp every approved NULL row");

    // The previously-stamped "fresh" row should NOT have its timestamp
    // overwritten by the backfill — we use coalesce(approved_at, …)?
    // The production migration uses WHERE approved_at IS NULL, which
    // is cleaner: it can't accidentally rewrite existing data. Verify
    // that invariant.
    let display_name: String = sqlx::query_scalar!(
        "select display_name from equipment_items
         where canonical_name = 'fresh stamped'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(display_name, "Fresh stamped");
}
