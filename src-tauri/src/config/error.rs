#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not determine config directory")]
    NoConfigDir,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),

    #[error("Cave error: {0}")]
    Cave(#[from] crate::cave::CaveError),

    #[error("Internal state error: mutex poisoned")]
    Poisoned,
}

// Allow returning ConfigError from Tauri commands
impl serde::Serialize for ConfigError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
