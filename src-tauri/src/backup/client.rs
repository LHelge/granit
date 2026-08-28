//! HTTP client for the backup backend (`granit-server`).

use std::time::Duration;

use granit_api::{
    ApiErrorBody, BackupInfo, CreateBackupRequest, CreateBackupResponse, ListBackupsResponse,
};
use uuid::Uuid;

use super::BackupError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Generous overall timeout: covers uploading a whole archive on a slow link.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub(crate) struct BackupApiClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl BackupApiClient {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Result<Self, BackupError> {
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

    pub(crate) async fn create_backup(
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

    pub(crate) async fn complete_backup(&self, id: Uuid) -> Result<BackupInfo, BackupError> {
        let response = self
            .http
            .post(format!("{}/api/v1/backups/{id}/complete", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        Ok(Self::expect_success(response).await?.json().await?)
    }

    pub(crate) async fn list_backups(&self) -> Result<Vec<BackupInfo>, BackupError> {
        let response = self
            .http
            .get(format!("{}/api/v1/backups", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let listed: ListBackupsResponse = Self::expect_success(response).await?.json().await?;
        Ok(listed.backups)
    }

    /// Upload the encrypted archive to the presigned URL. Deliberately a
    /// plain PUT with no auth header — the signature is in the URL.
    pub(crate) async fn upload(
        &self,
        presigned_url: &str,
        body: Vec<u8>,
    ) -> Result<(), BackupError> {
        let response = self.http.put(presigned_url).body(body).send().await?;
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
