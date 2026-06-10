//! Rebuild `photos.filters` (the legacy comma-joined display-names cache)
//! from the `photo_filters` junction. Called by every writer of the
//! junction in the same transaction.
//!
//! Junction is the source of truth; cache exists for the older browse
//! indexes that still scan `photos.filters` directly.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;

/// Recompute the cache string for `photo_id`. Empty junction → NULL cache.
pub async fn rebuild(tx: &mut Transaction<'_, Postgres>, photo_id: Uuid) -> Result<(), AppError> {
    let rows = sqlx::query!(
        r#"select e.display_name as "display_name!"
             from photo_filters pf
             join equipment_items e on e.id = pf.item_id
            where pf.photo_id = $1
            order by pf.position, e.display_name"#,
        photo_id
    )
    .fetch_all(&mut **tx)
    .await?;

    let joined: Option<String> = if rows.is_empty() {
        None
    } else {
        Some(
            rows.iter()
                .map(|r| r.display_name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    sqlx::query!(
        "update photos set filters = $1 where id = $2",
        joined,
        photo_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Recompute the cache string for EVERY photo referencing `item_id` through
/// the junction. One set-based UPDATE — used after an equipment item rename,
/// where the per-photo [`rebuild`] would be O(referencing photos) round trips.
/// The aggregation order matches `rebuild` exactly (`pf.position,
/// e.display_name`) so both paths produce identical strings.
pub async fn rebuild_for_item(
    tx: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"update photos p
              set filters = sub.s
             from (select pf.photo_id,
                          string_agg(e.display_name, ', '
                                     order by pf.position, e.display_name) as s
                     from photo_filters pf
                     join equipment_items e on e.id = pf.item_id
                    where pf.photo_id in (select photo_id from photo_filters
                                           where item_id = $1)
                    group by pf.photo_id) sub
            where p.id = sub.photo_id"#,
        item_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
