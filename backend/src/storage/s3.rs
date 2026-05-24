use async_trait::async_trait;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Builder, Credentials, Region},
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{CreateBucketConfiguration, Delete, ObjectIdentifier},
};
use bytes::Bytes;
use std::time::Duration;

use super::Storage;
use crate::AppError;

pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub async fn new(
        endpoint: Option<&str>,
        region: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
        path_style: bool,
    ) -> Result<Self, AppError> {
        let creds = Credentials::new(access_key, secret_key, None, None, "static");
        let region_owned = Region::new(region.to_string());
        let mut builder = Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(region_owned)
            .force_path_style(path_style);
        if let Some(ep) = endpoint {
            builder = builder.endpoint_url(ep);
        }
        let client = Client::from_conf(builder.build());
        let s = S3Storage {
            client,
            bucket: bucket.to_string(),
        };
        s.ensure_bucket().await?;
        Ok(s)
    }

    /// Create the bucket if it doesn't already exist. Idempotent.
    async fn ensure_bucket(&self) -> Result<(), AppError> {
        let head = self.client.head_bucket().bucket(&self.bucket).send().await;
        if head.is_ok() {
            tracing::info!(bucket = %self.bucket, "S3 bucket already exists");
            return Ok(());
        }
        tracing::info!(bucket = %self.bucket, "S3 bucket not found, creating");
        // Try to create. MinIO does not require LocationConstraint;
        // most S3 regions do, except us-east-1 where it must be omitted.
        let mut req = self.client.create_bucket().bucket(&self.bucket);
        let region = self
            .client
            .config()
            .region()
            .map(|r| r.as_ref().to_string())
            .unwrap_or_default();
        if !region.is_empty() && region != "us-east-1" {
            let cfg = CreateBucketConfiguration::builder()
                .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::from(
                    region.as_str(),
                ))
                .build();
            req = req.create_bucket_configuration(cfg);
        }
        req.send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("create bucket: {e}")))?;
        tracing::info!(bucket = %self.bucket, "S3 bucket created");
        Ok(())
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("s3 put: {e}")))
    }

    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError> {
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| AppError::Internal(format!("s3 read body: {e}")))?
                    .into_bytes();
                Ok(Some(bytes))
            }
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_no_such_key() {
                    Ok(None)
                } else {
                    Err(AppError::Internal(format!("s3 get: {svc_err}")))
                }
            }
        }
    }

    async fn get_range(&self, key: &str, start: u64, end: u64) -> Result<Option<Bytes>, AppError> {
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(format!("bytes={start}-{end}"))
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| AppError::Internal(format!("s3 read range body: {e}")))?
                    .into_bytes();
                Ok(Some(bytes))
            }
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_no_such_key() {
                    Ok(None)
                } else {
                    Err(AppError::Internal(format!("s3 get_range: {svc_err}")))
                }
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("s3 delete: {e}")))
    }

    async fn signed_url(&self, key: &str, ttl_secs: u64) -> Result<String, AppError> {
        let cfg = PresigningConfig::expires_in(Duration::from_secs(ttl_secs))
            .map_err(|e| AppError::Internal(format!("presigning config: {e}")))?;
        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(cfg)
            .await
            .map_err(|e| AppError::Internal(format!("s3 presign: {e}")))?;
        Ok(presigned.uri().to_string())
    }

    async fn presigned_put(
        &self,
        key: &str,
        content_type: &str,
        body_bytes: u64,
        ttl_secs: u64,
    ) -> Result<String, AppError> {
        let cfg = PresigningConfig::expires_in(Duration::from_secs(ttl_secs))
            .map_err(|e| AppError::Internal(format!("presign cfg: {e}")))?;
        let signed = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .content_length(body_bytes as i64)
            .presigned(cfg)
            .await
            .map_err(|e| AppError::Internal(format!("presign: {e}")))?;
        Ok(signed.uri().to_string())
    }

    async fn delete_objects(&self, keys: &[String]) -> Result<(), AppError> {
        for chunk in keys.chunks(1000) {
            let objects: Result<Vec<ObjectIdentifier>, _> = chunk
                .iter()
                .map(|k| {
                    ObjectIdentifier::builder()
                        .key(k)
                        .build()
                        .map_err(|e| AppError::Internal(format!("s3 object identifier: {e}")))
                })
                .collect();
            let delete = Delete::builder()
                .set_objects(Some(objects?))
                .build()
                .map_err(|e| AppError::Internal(format!("s3 delete spec: {e}")))?;
            let output = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("s3 delete_objects: {e}")))?;

            // aws-sdk-s3 returns Ok for the request even when individual objects fail.
            // Surface partial failures as an AppError so the caller (purge_one_user) treats
            // the user as not-yet-fully-deleted and the next hourly tick retries.
            let errors = output.errors();
            if !errors.is_empty() {
                let count = errors.len();
                let first = errors
                    .first()
                    .map(|e| format!("{:?}", e))
                    .unwrap_or_default();
                return Err(AppError::Internal(format!(
                    "s3 delete_objects partial failure: {count} object(s) errored, first: {first}"
                )));
            }
        }
        Ok(())
    }
}
