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
    .fetch_all(&pool).await.unwrap();
    assert_eq!(row.len(), 4, "expected 4 new equipment_items columns, got {}", row.len());

    // 5 specs sub-tables exist
    for t in ["telescope_specs","camera_specs","filter_specs","mount_specs","focal_modifier_specs"] {
        let exists: bool = sqlx::query_scalar!(
            "select exists(select 1 from information_schema.tables where table_name=$1)", t
        ).fetch_one(&pool).await.unwrap().unwrap_or(false);
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
    ).fetch_all(&pool).await.unwrap().into_iter().flatten().collect();
    assert_eq!(pk_cols, vec!["photo_id".to_string(), "item_id".to_string()]);
}
