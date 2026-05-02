//! Object storage abstraction. S3-compatible (R2 in prod, MinIO in dev).
//! In-memory impl for tests.

use async_trait::async_trait;
use bytes::Bytes;

use crate::AppError;

pub mod memory;
pub mod s3;

pub use memory::MemoryStorage;
pub use s3::S3Storage;

#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store `body` at `key` with the given content type. Overwrites if exists.
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError>;

    /// Retrieve an object's bytes. None if missing.
    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError>;

    /// Delete an object. Idempotent (no error if missing).
    async fn delete(&self, key: &str) -> Result<(), AppError>;

    /// Batch-delete objects by key. Unknown keys are silently skipped.
    async fn delete_objects(&self, keys: &[String]) -> Result<(), AppError>;

    /// Generate a pre-signed URL for a GET on `key`, valid for `ttl_secs` seconds.
    /// Maximum TTL is 7 days (604 800 s). Tests use `MemoryStorage` which returns
    /// `memory://{key}` — callers must not follow this URL.
    async fn signed_url(&self, key: &str, ttl_secs: u64) -> Result<String, AppError>;
}
