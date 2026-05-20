//! Migration 0018: typed specs sub-tables + photo_filters junction.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

#[tokio::test]
async fn migration_0018_creates_tables_and_columns() {
    let (_app, pool) = common::make_app_and_pool().await;

    // catalog metadata columns
    let row = sqlx::query!(
        r#"select column_name from information_schema.columns
            where table_name='equipment_items'
              and column_name in ('status','submitted_by','approved_at','created_at')
            order by column_name"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        row.len(),
        4,
        "expected 4 new equipment_items columns, got {}",
        row.len()
    );

    // 5 specs sub-tables exist
    for t in [
        "telescope_specs",
        "camera_specs",
        "filter_specs",
        "mount_specs",
        "focal_modifier_specs",
    ] {
        let exists: bool = sqlx::query_scalar!(
            "select exists(select 1 from information_schema.tables where table_name=$1)",
            t
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap_or(false);
        assert!(exists, "{t} table missing");
    }

    // photo_filters junction with composite PK
    let pk_cols: Vec<String> = sqlx::query_scalar!(
        r#"select kcu.column_name
             from information_schema.table_constraints tc
             join information_schema.key_column_usage kcu
               on tc.constraint_name = kcu.constraint_name
            where tc.table_name='photo_filters' and tc.constraint_type='PRIMARY KEY'
            order by kcu.ordinal_position"#
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .flatten()
    .collect();
    assert_eq!(pk_cols, vec!["photo_id".to_string(), "item_id".to_string()]);
}

#[tokio::test]
async fn backfill_populates_photo_filters_from_legacy_string() {
    let (_app, pool) = common::make_app_and_pool().await;

    let owner_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into users
               (email, handle, display_name, password_hash, email_verified_at, created_at)
            values ('bob@example.com', 'bob1', 'Bob', 'x', now(), now())
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let r_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('filter','r','R',1,'approved',now(),'','R')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let g_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, usage_count, status, approved_at,
                 brand, model)
            values ('filter','g','G',1,'approved',now(),'','G')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Insert a photo with legacy comma-joined filters. The third token
    // ("Astronomik CLS") has no matching equipment_items row and must
    // be silently dropped by the backfill statement.
    let photo_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into photos
                (owner_id, storage_key, original_name, bytes, mime, status,
                 short_id, original_uploaded_at, filters)
            values ($1, 'k/x', 'x.tif', 1, 'image/tiff', 'ready',
                    'BACKFILL', now(), 'R, G, Astronomik CLS')
            returning id"#,
        owner_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Replay the migration's backfill statement to assert idempotency
    // (ON CONFLICT DO NOTHING means re-running this against rows already
    // populated by the migration must be a no-op).
    sqlx::query!(
        r#"insert into photo_filters (photo_id, item_id, position)
            select s.photo_id, e.id, s.position::smallint
              from (
                select p.id as photo_id,
                       btrim(t.token) as token,
                       t.ord - 1 as position
                  from photos p,
                       unnest(string_to_array(p.filters, ',')) with ordinality as t(token, ord)
                 where p.filters is not null
                   and length(btrim(p.filters)) > 0
              ) s
              join equipment_items e
                on e.kind = 'filter'
               and e.canonical_name = lower(s.token)
            on conflict do nothing"#
    )
    .execute(&pool)
    .await
    .unwrap();

    let pairs: Vec<(uuid::Uuid, i16)> = sqlx::query!(
        "select item_id, position from photo_filters
          where photo_id = $1
          order by position",
        photo_id
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.item_id, r.position))
    .collect();

    assert_eq!(
        pairs.len(),
        2,
        "orphan token must not produce a junction row"
    );
    assert_eq!(pairs[0], (r_id, 0));
    assert_eq!(pairs[1], (g_id, 1));
}
