use astrophoto::storage::{MemoryStorage, Storage};
use std::sync::Arc;

#[tokio::test]
async fn memory_presign_returns_synthetic_url() {
    let s: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
    let url = s
        .presigned_put("originals/abc", "image/jpeg", 1024, 60)
        .await
        .unwrap();
    assert!(url.starts_with("memory://"));
}
