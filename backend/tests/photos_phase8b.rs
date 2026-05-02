//! Integration tests for Phase 8b: drafts, replace, my-photos stats,
//! visibility predicate. Phase 5 upload tests stay in `photos.rs`.

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use uuid::Uuid;

use astrophoto::db;

#[allow(clippy::unwrap_used)]
async fn test_pool() -> (sqlx::PgPool, testcontainers::ContainerAsync<PgImage>) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    (pool, pg)
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn is_visible_to_returns_true_for_published_to_anyone() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O'), ($3, $4, '', 'V')",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e")
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, published_at, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'ready', now(), now(), 'caption')
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer))
            .await
            .unwrap()
    );
    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, None)
            .await
            .unwrap()
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn is_visible_to_returns_false_for_draft_to_non_owner_and_anon() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O'), ($3, $4, '', 'V')",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e")
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload')
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        !astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer))
            .await
            .unwrap()
    );
    assert!(
        !astrophoto::photos::queries::is_visible_to(&pool, photo_id, None)
            .await
            .unwrap()
    );
    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(owner))
            .await
            .unwrap()
    );
}
