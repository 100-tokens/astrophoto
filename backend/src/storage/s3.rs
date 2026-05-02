use async_trait::async_trait;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Builder, Credentials, Region},
    primitives::ByteStream,
    types::CreateBucketConfiguration,
};
use bytes::Bytes;

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
}
