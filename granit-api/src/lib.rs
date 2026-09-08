//! Wire types for the Granit backup backend REST API.
//!
//! Shared between the desktop app (`granit`) and the backup server
//! (`granit-server`). This crate must stay wasm-compatible: the frontend
//! (`granit-ui`) links it to render backup listings, so dependencies are
//! limited to serde-adjacent crates.
//!
//! All endpoints live under `/api/v1` and authenticate with
//! `Authorization: Bearer grnt_<token>`. Errors are returned as
//! [`ApiErrorBody`] with an appropriate HTTP status code.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Request body for `POST /api/v1/backups`.
///
/// Announces a new snapshot before upload. Sizes and digests refer to the
/// encrypted archive (ciphertext), which is all the server ever sees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    /// Display name of the cave being backed up.
    pub cave_name: String,
    /// Size of the encrypted archive in bytes.
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 digest of the encrypted archive.
    pub sha256_hex: String,
}

/// Response body for `POST /api/v1/backups`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBackupResponse {
    /// The freshly created backup record (state [`BackupState::Pending`]).
    pub backup: BackupInfo,
    /// Presigned S3 PUT URL; send the archive with `upload_headers`.
    pub upload_url: String,
    /// Required signed headers, including size, checksum and write condition.
    #[serde(default)]
    pub upload_headers: BTreeMap<String, String>,
    /// When the presigned URL stops being valid.
    pub upload_expires_at: DateTime<Utc>,
}

/// One backup snapshot record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: Uuid,
    pub cave_name: String,
    /// Size of the encrypted archive in bytes.
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 digest of the encrypted archive.
    pub sha256_hex: String,
    pub state: BackupState,
    pub created_at: DateTime<Utc>,
    /// Set once the upload has been verified against the object store.
    pub completed_at: Option<DateTime<Utc>>,
}

/// Lifecycle state of a backup record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupState {
    /// Record created, upload not yet verified.
    Pending,
    /// Upload verified against the object store.
    Complete,
}

/// Response body for `GET /api/v1/backups`, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBackupsResponse {
    pub backups: Vec<BackupInfo>,
}

/// Response body for `GET /api/v1/backups/{id}/download`.
///
/// Only [`BackupState::Complete`] snapshots can be downloaded; requesting a
/// pending one yields 409 Conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadBackupResponse {
    /// Presigned S3 GET URL; fetch the encrypted archive with a plain HTTP GET.
    pub download_url: String,
    /// When the presigned URL stops being valid.
    pub download_expires_at: DateTime<Utc>,
}

/// Uniform JSON error body for all non-2xx responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backup() -> BackupInfo {
        BackupInfo {
            id: Uuid::nil(),
            cave_name: "my-cave".to_string(),
            size_bytes: 42,
            sha256_hex: "ab".repeat(32),
            state: BackupState::Pending,
            created_at: DateTime::parse_from_rfc3339("2026-08-28T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            completed_at: None,
        }
    }

    #[test]
    fn backup_state_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&BackupState::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&BackupState::Complete).unwrap(),
            "\"complete\""
        );
    }

    #[test]
    fn create_backup_round_trip() {
        let req = CreateBackupRequest {
            cave_name: "my-cave".to_string(),
            size_bytes: 42,
            sha256_hex: "ab".repeat(32),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(
            serde_json::from_str::<CreateBackupRequest>(&json).unwrap(),
            req
        );

        let resp = CreateBackupResponse {
            backup: sample_backup(),
            upload_url: "http://localhost:8080/granit-backups/backups/x.grnt?sig=y".to_string(),
            upload_headers: BTreeMap::from([("if-none-match".into(), "*".into())]),
            upload_expires_at: DateTime::parse_from_rfc3339("2026-08-28T12:15:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            serde_json::from_str::<CreateBackupResponse>(&json).unwrap(),
            resp
        );
    }

    #[test]
    fn list_backups_round_trip() {
        let resp = ListBackupsResponse {
            backups: vec![sample_backup()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            serde_json::from_str::<ListBackupsResponse>(&json).unwrap(),
            resp
        );
    }

    #[test]
    fn download_backup_round_trip() {
        let resp = DownloadBackupResponse {
            download_url: "http://localhost:8080/granit-backups/backups/x.grnt?sig=y".to_string(),
            download_expires_at: DateTime::parse_from_rfc3339("2026-08-28T12:15:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            serde_json::from_str::<DownloadBackupResponse>(&json).unwrap(),
            resp
        );
    }

    #[test]
    fn error_body_round_trip() {
        let body = ApiErrorBody {
            message: "invalid API key".to_string(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"message":"invalid API key"}"#);
        assert_eq!(serde_json::from_str::<ApiErrorBody>(&json).unwrap(), body);
    }
}
