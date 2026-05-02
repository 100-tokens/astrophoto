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
}
