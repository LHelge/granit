use std::time::Duration;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use chrono::{DateTime, Utc};

use crate::config::S3Config;
use crate::error::ServerError;

/// How long a presigned upload URL stays valid.
pub const UPLOAD_URL_TTL: Duration = Duration::from_secs(15 * 60);

/// Object storage access for backup archives.
///
/// Wraps two S3 clients built from the same credentials: `internal` talks to
/// the store directly (docker-internal endpoint) for bucket bootstrap and
/// object verification, while `presign` signs URLs against the public origin
/// so the resulting links are reachable from the desktop app.
pub struct Storage {
    bucket: String,
    inner: StorageInner,
}

enum StorageInner {
    S3 {
        internal: aws_sdk_s3::Client,
        presign: aws_sdk_s3::Client,
    },
    /// Test stand-in: presigns deterministic URLs and reports a configurable
    /// object size, so handler tests need no live object store.
    #[cfg(test)]
    Stub {
        head_size: std::sync::Mutex<Option<u64>>,
    },
}

fn s3_client(cfg: &S3Config, endpoint: &str) -> aws_sdk_s3::Client {
    let credentials = Credentials::new(
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
        None,
        None,
        "granit-server",
    );
    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url(endpoint)
        .credentials_provider(credentials)
        // RustFS serves buckets path-style, like MinIO.
        .force_path_style(true)
        .build();
    aws_sdk_s3::Client::from_conf(config)
}

impl Storage {
    pub fn new(cfg: &S3Config) -> Self {
        Self {
            bucket: cfg.bucket.clone(),
            inner: StorageInner::S3 {
                internal: s3_client(cfg, &cfg.endpoint),
                presign: s3_client(cfg, &cfg.public_endpoint),
            },
        }
    }

    #[cfg(test)]
    pub fn stub() -> Self {
        Self {
            bucket: "granit-backups".to_string(),
            inner: StorageInner::Stub {
                head_size: std::sync::Mutex::new(None),
            },
        }
    }

    /// Set the object size the stub reports for any key (`None` = object missing).
    #[cfg(test)]
    pub fn stub_set_head_size(&self, size: Option<u64>) {
        if let StorageInner::Stub { head_size } = &self.inner {
            *head_size.lock().unwrap() = size;
        }
    }

    /// Create the bucket if it does not exist yet. Idempotent.
    pub async fn ensure_bucket(&self) -> Result<(), ServerError> {
        match &self.inner {
            StorageInner::S3 { internal, .. } => {
                match internal.create_bucket().bucket(&self.bucket).send().await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        let service_err = err.into_service_error();
                        if service_err.is_bucket_already_owned_by_you()
                            || service_err.is_bucket_already_exists()
                        {
                            Ok(())
                        } else {
                            Err(ServerError::S3(service_err.to_string()))
                        }
                    }
                }
            }
            #[cfg(test)]
            StorageInner::Stub { .. } => Ok(()),
        }
    }

    /// Presign a PUT for `key`, returning the URL and its expiry time.
    pub async fn presign_put(&self, key: &str) -> Result<(String, DateTime<Utc>), ServerError> {
        let expires_at = Utc::now() + UPLOAD_URL_TTL;
        match &self.inner {
            StorageInner::S3 { presign, .. } => {
                let presigning = PresigningConfig::expires_in(UPLOAD_URL_TTL)
                    .map_err(|err| ServerError::S3(err.to_string()))?;
                let request = presign
                    .put_object()
                    .bucket(&self.bucket)
                    .key(key)
                    .presigned(presigning)
                    .await
                    .map_err(|err| ServerError::S3(err.to_string()))?;
                Ok((request.uri().to_string(), expires_at))
            }
            #[cfg(test)]
            StorageInner::Stub { .. } => Ok((
                format!("http://stub.local/{}/{key}?sig=test", self.bucket),
                expires_at,
            )),
        }
    }

    /// Size of the object at `key`, or `None` if it does not exist.
    pub async fn head_size(&self, key: &str) -> Result<Option<u64>, ServerError> {
        match &self.inner {
            StorageInner::S3 { internal, .. } => {
                match internal
                    .head_object()
                    .bucket(&self.bucket)
                    .key(key)
                    .send()
                    .await
                {
                    Ok(head) => Ok(head.content_length().map(|len| len.max(0) as u64)),
                    Err(err) => {
                        let service_err = err.into_service_error();
                        if service_err.is_not_found() {
                            Ok(None)
                        } else {
                            Err(ServerError::S3(service_err.to_string()))
                        }
                    }
                }
            }
            #[cfg(test)]
            StorageInner::Stub { head_size } => Ok(*head_size.lock().unwrap()),
        }
    }
}
