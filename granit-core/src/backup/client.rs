//! HTTP client for the backup backend (`granit-server`), and the
//! [`BackupApi`] seam the backup and restore flows talk to it through.

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use granit_api::{
    ApiErrorBody, BackupInfo, CreateBackupRequest, CreateBackupResponse, DownloadBackupResponse,
    ListBackupsResponse,
};
use uuid::Uuid;

use super::BackupError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Generous overall timeout: covers uploading a whole archive on a slow link.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The backup backend as the backup and restore flows ([`super::run`]) use
/// it: [`BackupApiClient`] in the app, an in-memory fake in tests.
pub trait BackupApi: Send + Sync {
    /// Announce a snapshot; the answer carries the presigned upload URL.
    fn create_backup(
        &self,
        request: &CreateBackupRequest,
    ) -> impl Future<Output = Result<CreateBackupResponse, BackupError>> + Send;

    /// Upload the encrypted archive to a presigned URL.
    fn upload(
        &self,
        presigned_url: &str,
        headers: &BTreeMap<String, String>,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<(), BackupError>> + Send;

    /// Have the backend verify the upload and mark the snapshot complete.
    fn complete_backup(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<BackupInfo, BackupError>> + Send;

    /// Every snapshot of the API key, newest first.
    fn list_backups(&self) -> impl Future<Output = Result<Vec<BackupInfo>, BackupError>> + Send;

    /// Delete a snapshot (object and record).
    fn delete_backup(&self, id: Uuid) -> impl Future<Output = Result<(), BackupError>> + Send;

    /// A presigned download URL for a complete snapshot.
    fn download_backup(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<DownloadBackupResponse, BackupError>> + Send;

    /// Fetch the encrypted archive from a presigned URL.
    fn download(
        &self,
        presigned_url: &str,
    ) -> impl Future<Output = Result<Vec<u8>, BackupError>> + Send;
}

pub struct BackupApiClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl BackupApiClient {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, BackupError> {
        let base_url = base_url.trim().trim_end_matches('/').to_string();
        let api_key = api_key.trim().to_string();
        if base_url.is_empty() || api_key.is_empty() {
            return Err(BackupError::NotConfigured);
        }
        let http = reqwest::Client::builder()
            .user_agent("Granit/1.0")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    async fn expect_success(response: reqwest::Response) -> Result<reqwest::Response, BackupError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        // The backend answers with a uniform JSON error body; fall back to
        // the raw text for anything else in the path (e.g. a proxy page).
        let message = match response.json::<ApiErrorBody>().await {
            Ok(body) => body.message,
            Err(_) => status
                .canonical_reason()
                .unwrap_or("unexpected response")
                .to_string(),
        };
        Err(BackupError::Api {
            status: status.as_u16(),
            message,
        })
    }
}

impl BackupApi for BackupApiClient {
    async fn create_backup(
        &self,
        request: &CreateBackupRequest,
    ) -> Result<CreateBackupResponse, BackupError> {
        let response = self
            .http
            .post(format!("{}/api/v1/backups", self.base_url))
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?;
        Ok(Self::expect_success(response).await?.json().await?)
    }

    async fn complete_backup(&self, id: Uuid) -> Result<BackupInfo, BackupError> {
        let response = self
            .http
            .post(format!("{}/api/v1/backups/{id}/complete", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        Ok(Self::expect_success(response).await?.json().await?)
    }

    async fn list_backups(&self) -> Result<Vec<BackupInfo>, BackupError> {
        let response = self
            .http
            .get(format!("{}/api/v1/backups", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let listed: ListBackupsResponse = Self::expect_success(response).await?.json().await?;
        Ok(listed.backups)
    }

    async fn delete_backup(&self, id: Uuid) -> Result<(), BackupError> {
        let response = self
            .http
            .delete(format!("{}/api/v1/backups/{id}", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        Self::expect_success(response).await?;
        Ok(())
    }

    async fn download_backup(&self, id: Uuid) -> Result<DownloadBackupResponse, BackupError> {
        let response = self
            .http
            .get(format!("{}/api/v1/backups/{id}/download", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        Ok(Self::expect_success(response).await?.json().await?)
    }

    /// Like `upload`, a plain request with no auth header — the signature
    /// is in the URL.
    async fn download(&self, presigned_url: &str) -> Result<Vec<u8>, BackupError> {
        let response = self.http.get(presigned_url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(BackupError::Api {
                status: status.as_u16(),
                message: "download from object storage was rejected".to_string(),
            });
        }
        Ok(response.bytes().await?.to_vec())
    }

    /// Deliberately a plain PUT with no auth header — the signature is in
    /// the URL.
    async fn upload(
        &self,
        presigned_url: &str,
        headers: &BTreeMap<String, String>,
        body: Vec<u8>,
    ) -> Result<(), BackupError> {
        let mut request = self.http.put(presigned_url).body(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(BackupError::Api {
                status: status.as_u16(),
                message: "upload to object storage was rejected".to_string(),
            });
        }
        Ok(())
    }
}
