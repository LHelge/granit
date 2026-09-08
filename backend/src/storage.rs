use std::collections::BTreeMap;
use std::time::Duration;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::ChecksumMode;
use base64::Engine;
use chrono::{DateTime, Utc};

use crate::config::S3Config;
use crate::error::ServerError;

/// How long presigned upload and download URLs stay valid.
pub const PRESIGN_URL_TTL: Duration = Duration::from_secs(15 * 60);

pub struct SignedUpload {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredObject {
    pub size_bytes: u64,
    pub checksum_sha256: Option<String>,
}

/// S3 expects base64, while the API and catalog use canonical lowercase hex.
pub fn checksum_sha256(sha256_hex: &str) -> Result<String, ServerError> {
    if sha256_hex.len() != 64
        || !sha256_hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ServerError::BadRequest(
            "sha256_hex must be 64 lowercase hexadecimal characters".into(),
        ));
    }
    let digest = hex::decode(sha256_hex)
        .map_err(|_| ServerError::BadRequest("invalid sha256_hex".into()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(digest))
}

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
        object: std::sync::Mutex<Option<StoredObject>>,
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
                object: std::sync::Mutex::new(None),
            },
        }
    }

    /// Set the object metadata the stub reports for any key.
    #[cfg(test)]
    pub fn stub_set_object(&self, stored: Option<StoredObject>) {
        if let StorageInner::Stub { object } = &self.inner {
            *object.lock().unwrap() = stored;
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

    /// Bind a write-once PUT to the exact announced ciphertext.
    pub async fn presign_put(
        &self,
        key: &str,
        size_bytes: i64,
        checksum: &str,
    ) -> Result<SignedUpload, ServerError> {
        let expires_at = Utc::now() + PRESIGN_URL_TTL;
        match &self.inner {
            StorageInner::S3 { presign, .. } => {
                let presigning = PresigningConfig::expires_in(PRESIGN_URL_TTL)
                    .map_err(|err| ServerError::S3(err.to_string()))?;
                let request = presign
                    .put_object()
                    .bucket(&self.bucket)
                    .key(key)
                    .content_length(size_bytes)
                    .checksum_sha256(checksum)
                    .if_none_match("*")
                    .presigned(presigning)
                    .await
                    .map_err(|err| ServerError::S3(err.to_string()))?;
                Ok(SignedUpload {
                    url: request.uri().to_string(),
                    headers: request
                        .headers()
                        .filter(|(name, _)| !name.eq_ignore_ascii_case("host"))
                        .map(|(name, value)| (name.to_string(), value.to_string()))
                        .collect(),
                    expires_at,
                })
            }
            #[cfg(test)]
            StorageInner::Stub { .. } => Ok(SignedUpload {
                url: format!("http://stub.local/{}/{key}?sig=test", self.bucket),
                headers: BTreeMap::from([
                    ("content-length".into(), size_bytes.to_string()),
                    ("x-amz-checksum-sha256".into(), checksum.into()),
                    ("if-none-match".into(), "*".into()),
                ]),
                expires_at,
            }),
        }
    }

    /// Presign a GET for `key`, returning the URL and its expiry time.
    pub async fn presign_get(&self, key: &str) -> Result<(String, DateTime<Utc>), ServerError> {
        let expires_at = Utc::now() + PRESIGN_URL_TTL;
        match &self.inner {
            StorageInner::S3 { presign, .. } => {
                let presigning = PresigningConfig::expires_in(PRESIGN_URL_TTL)
                    .map_err(|err| ServerError::S3(err.to_string()))?;
                let request = presign
                    .get_object()
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

    /// Delete the object at `key`. Deleting a missing object is not an
    /// error — S3 DeleteObject is idempotent, and a dangling row whose
    /// upload never happened must still be removable.
    pub async fn delete_object(&self, key: &str) -> Result<(), ServerError> {
        match &self.inner {
            StorageInner::S3 { internal, .. } => internal
                .delete_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
                .map(|_| ())
                .map_err(|err| ServerError::S3(err.into_service_error().to_string())),
            #[cfg(test)]
            StorageInner::Stub { .. } => Ok(()),
        }
    }

    /// Verified object checksum and size, or `None` if it does not exist.
    pub async fn head_object(&self, key: &str) -> Result<Option<StoredObject>, ServerError> {
        match &self.inner {
            StorageInner::S3 { internal, .. } => {
                match internal
                    .head_object()
                    .checksum_mode(ChecksumMode::Enabled)
                    .bucket(&self.bucket)
                    .key(key)
                    .send()
                    .await
                {
                    Ok(head) => Ok(head.content_length().map(|len| StoredObject {
                        size_bytes: len.max(0) as u64,
                        checksum_sha256: head.checksum_sha256().map(str::to_string),
                    })),
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
            StorageInner::Stub { object } => Ok(object.lock().unwrap().clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run against a dedicated local RustFS instance; never a production bucket.
    #[tokio::test]
    #[ignore = "requires an isolated S3 server and GRANIT_TEST_S3_ENDPOINT"]
    async fn object_store_enforces_upload_constraints() {
        use sha2::{Digest, Sha256};
        let endpoint = std::env::var("GRANIT_TEST_S3_ENDPOINT").expect("test S3 endpoint");
        let storage = Storage::new(&S3Config {
            endpoint: endpoint.clone(),
            public_endpoint: endpoint,
            bucket: format!("granit-audit-{}", uuid::Uuid::new_v4()),
            access_key: std::env::var("GRANIT_TEST_S3_ACCESS_KEY").expect("test S3 access key"),
            secret_key: std::env::var("GRANIT_TEST_S3_SECRET_KEY").expect("test S3 secret key"),
        });
        storage.ensure_bucket().await.unwrap();
        let body = b"encrypted test archive";
        let checksum = checksum_sha256(&hex::encode(Sha256::digest(body))).unwrap();
        let upload = storage
            .presign_put("snapshot.grnt", body.len() as i64, &checksum)
            .await
            .unwrap();
        let http = reqwest::Client::new();
        let put = |payload: Vec<u8>| {
            let mut request = http.put(&upload.url).body(payload);
            for (name, value) in &upload.headers {
                request = request.header(name, value);
            }
            request
        };
        // A matching length with different bytes must fail before creating an object.
        let bad = put(vec![b'x'; body.len()]).send().await.unwrap();
        assert!(bad.status().is_client_error(), "{}", bad.status());
        assert_eq!(storage.head_object("snapshot.grnt").await.unwrap(), None);
        // Removing the condition invalidates the signature.
        let mut unsigned = http.put(&upload.url).body(body.to_vec());
        for (name, value) in &upload.headers {
            if name != "if-none-match" {
                unsigned = unsigned.header(name, value);
            }
        }
        assert!(unsigned.send().await.unwrap().status().is_client_error());
        let oversized = vec![b'x'; body.len() + 1];
        let oversized_len = oversized.len();
        let wrong_size = put(oversized)
            .header("content-length", oversized_len)
            .send()
            .await
            .unwrap();
        assert!(wrong_size.status().is_client_error());
        let response = put(body.to_vec()).send().await.unwrap();
        assert!(
            response.status().is_success(),
            "{}: {}",
            response.status(),
            response.text().await.unwrap()
        );
        assert_eq!(
            storage.head_object("snapshot.grnt").await.unwrap(),
            Some(StoredObject {
                size_bytes: body.len() as u64,
                checksum_sha256: Some(checksum),
            })
        );
        // Even replaying the identical upload must not overwrite the object.
        assert_eq!(
            put(body.to_vec()).send().await.unwrap().status(),
            reqwest::StatusCode::PRECONDITION_FAILED
        );
        storage.delete_object("snapshot.grnt").await.unwrap();
        if let StorageInner::S3 { internal, .. } = &storage.inner {
            internal
                .delete_bucket()
                .bucket(&storage.bucket)
                .send()
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn upload_signature_covers_size_checksum_and_write_condition() {
        let storage = Storage::new(&S3Config {
            endpoint: "http://localhost:9000".into(),
            public_endpoint: "https://storage.example.test".into(),
            bucket: "granit-backups".into(),
            access_key: "test-access-key".into(),
            secret_key: "test-secret-key".into(),
        });
        let checksum = checksum_sha256(&"ab".repeat(32)).unwrap();
        let upload = storage
            .presign_put("backups/test.grnt", 1234, &checksum)
            .await
            .unwrap();
        assert_eq!(upload.headers["content-length"], "1234");
        assert_eq!(upload.headers["if-none-match"], "*");
        assert!(upload.url.contains("content-length"), "size must be signed");
        assert!(
            upload.url.contains("if-none-match"),
            "write condition must be signed"
        );
        // The SDK may place the checksum in a signed header or in the signed
        // query string; either must remain bound to the request.
        assert!(upload.url.contains("x-amz-checksum-sha256"));
        if let Some(header) = upload.headers.get("x-amz-checksum-sha256") {
            assert_eq!(header, &checksum);
        }
    }
}
