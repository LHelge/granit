//! Cave backup: pack the cave into a compressed, encrypted archive and
//! upload it to the self-hosted backup backend (see `backend/`).
//!
//! The encryption key is derived from a user passphrase with Argon2id.
//! Salt and KDF parameters travel unencrypted in every archive header, so
//! restoring on a fresh machine needs only the passphrase and the archive;
//! the derived key is cached in `.granit/backup.key` so routine backups
//! never prompt. The passphrase itself is never stored and never leaves
//! the machine.
//!
//! The flows a host runs (back up the open cave, restore a snapshot, list
//! and delete snapshots, set the passphrase) are in [`run`], with the
//! [`OperationGuard`] that keeps backups and restores from overlapping.

mod archive;
mod client;
mod crypto;
mod restore;
pub mod run;

pub use archive::pack_cave;
pub use client::{BackupApi, BackupApiClient};
pub use crypto::{encrypt, has_key_file, load_key, set_passphrase};
pub use granit_api::{BackupInfo, BackupState};
pub use restore::{restore_in_place, restore_to_new_dir, verify_and_decrypt};
pub use run::{
    backup_cave, backup_now, cave_has_key, delete_backup, list_backups, restore_backup,
    restore_with, set_cave_passphrase, OperationGuard, OperationTicket, RestoreFailure,
    RestoreRequest, Restored,
};

/// Errors from packing, encrypting, or uploading a backup.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("No cave is open")]
    NoCave,
    #[error("Backup is not configured — set the backend URL and API key in settings")]
    NotConfigured,
    #[error("No backup passphrase is set — set one in settings")]
    PassphraseNotSet,
    #[error("The passphrase must be at least {min} characters", min = crypto::MIN_PASSPHRASE_LEN)]
    WeakPassphrase,
    #[error("A backup or restore is already running")]
    AlreadyRunning,
    #[error("This archive needs its passphrase — the cave's cached key does not match")]
    PassphraseRequired,
    #[error("Downloaded archive is corrupted — its checksum does not match the record")]
    ChecksumMismatch,
    #[error("The target folder is not empty")]
    TargetNotEmpty,
    #[error("Snapshot not found on the backend")]
    SnapshotNotFound,
    #[error("Another cave was opened during the restore — nothing was replaced")]
    CaveChanged,
    #[error("Restore finished, but opening the restored cave failed: {0}")]
    OpenRestored(String),
    #[error("Invalid backup key file: {0}")]
    InvalidKeyFile(String),
    #[error("Unsupported backup key derivation parameters")]
    UnsupportedKdfParams,
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Encryption failed")]
    Encrypt,
    #[error("Decryption failed — wrong passphrase or corrupted archive")]
    Decrypt,
    #[error("Backend request failed: {0}")]
    Http(String),
    #[error("Backend error ({status}): {message}")]
    Api { status: u16, message: String },
}

impl From<std::io::Error> for BackupError {
    fn from(err: std::io::Error) -> Self {
        BackupError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for BackupError {
    fn from(err: reqwest::Error) -> Self {
        BackupError::Http(err.to_string())
    }
}

impl serde::Serialize for BackupError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
