use serde::{Deserialize, Serialize};

/// Cave-local backup settings, stored in `.granit/config.yml`.
///
/// The encryption passphrase is deliberately absent: only the derived key
/// is cached, in `.granit/backup.key`, outside the config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Public origin of the backup backend (the Caddy origin), e.g.
    /// `https://backup.example.com`.
    #[serde(default)]
    pub backend_url: String,
    /// API key issued by `granit-server create-key`.
    #[serde(default)]
    pub api_key: String,
}

impl BackupConfig {
    /// Whether both fields needed to reach the backend are present.
    pub fn is_configured(&self) -> bool {
        !self.backend_url.trim().is_empty() && !self.api_key.trim().is_empty()
    }
}

/// Coarse progress stage of a running backup, sent with `backup:progress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupStage {
    Packing,
    Encrypting,
    Uploading,
}

/// Payload of the `backup:progress` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupProgress {
    pub stage: BackupStage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_configured_requires_both_fields() {
        assert!(!BackupConfig::default().is_configured());
        assert!(!BackupConfig {
            backend_url: "http://localhost:8080".into(),
            api_key: "  ".into(),
        }
        .is_configured());
        assert!(BackupConfig {
            backend_url: "http://localhost:8080".into(),
            api_key: "grnt_x".into(),
        }
        .is_configured());
    }

    #[test]
    fn progress_round_trips() {
        let progress = BackupProgress {
            stage: BackupStage::Encrypting,
        };
        let json = serde_json::to_string(&progress).unwrap();
        let back: BackupProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, progress);
    }
}
