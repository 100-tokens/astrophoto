use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Mutex;

use super::Storage;
use crate::AppError;

#[derive(Default)]
pub struct MemoryStorage {
    inner: Mutex<HashMap<String, (String, Bytes)>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        g.insert(key.to_string(), (content_type.to_string(), body));
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        Ok(g.get(key).map(|(_, b)| b.clone()))
    }

    async fn get_range(&self, key: &str, start: u64, end: u64) -> Result<Option<Bytes>, AppError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        Ok(g.get(key).map(|(_, b)| {
            let lo = (start as usize).min(b.len());
            let hi = (end as usize).saturating_add(1).min(b.len()); // inclusive end
            b.slice(lo..hi)
        }))
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        g.remove(key);
        Ok(())
    }

    async fn delete_objects(&self, keys: &[String]) -> Result<(), AppError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        for key in keys {
            g.remove(key.as_str());
        }
        Ok(())
    }

    async fn signed_url(&self, key: &str, _ttl_secs: u64) -> Result<String, AppError> {
        Ok(format!("memory://{key}"))
    }

    async fn presigned_put(
        &self,
        key: &str,
        _content_type: &str,
        _body_bytes: u64,
        _ttl_secs: u64,
    ) -> Result<String, AppError> {
        // Tests don't actually PUT against this URL; they use Storage::put
        // directly to seed objects.
        Ok(format!("memory://put/{key}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_get_delete_round_trip() {
        let s = MemoryStorage::new();
        let body = Bytes::from_static(b"hello");
        s.put("k", "text/plain", body.clone()).await.unwrap();
        assert_eq!(s.get("k").await.unwrap().unwrap(), body);
        s.delete("k").await.unwrap();
        assert!(s.get("k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_range_slices_inclusive() {
        let s = MemoryStorage::new();
        s.put(
            "k",
            "application/octet-stream",
            Bytes::from_static(b"0123456789"),
        )
        .await
        .unwrap();
        assert_eq!(
            s.get_range("k", 2, 5).await.unwrap().unwrap().as_ref(),
            b"2345"
        );
        // clamps past end-of-object
        assert_eq!(
            s.get_range("k", 8, 99).await.unwrap().unwrap().as_ref(),
            b"89"
        );
        assert!(s.get_range("missing", 0, 4).await.unwrap().is_none());
    }
}
