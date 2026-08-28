//! Cave backup: pack the cave into a compressed, encrypted archive and
//! upload it to the self-hosted backup backend (see `backend/`).
//!
//! The encryption key is derived from a user passphrase with Argon2id.
//! Salt and KDF parameters travel unencrypted in every archive header, so
//! restoring on a fresh machine needs only the passphrase and the archive;
//! the derived key is cached in `.granit/backup.key` so routine backups
//! never prompt. The passphrase itself is never stored and never leaves
//! the machine.

mod archive;
mod client;
mod crypto;

pub(crate) use archive::pack_cave;
pub(crate) use client::BackupApiClient;
pub(crate) use crypto::{encrypt, has_key_file, load_key, set_passphrase};

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
    #[error("A backup is already running")]
    AlreadyRunning,
    #[error("Invalid backup key file: {0}")]
    InvalidKeyFile(String),
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
