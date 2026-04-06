mod error;

use std::path::{Path, PathBuf};

pub use error::ConfigError;
use granit_types::AppConfig;

/// Load config from the current storage path.
/// Returns defaults if the file does not exist; missing fields fall back to defaults.
pub fn load() -> Result<AppConfig, ConfigError> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let mut config: AppConfig = serde_yml::from_str(&contents)?;
            config.active_cave = None;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(e.into()),
    }
}

/// Persist this config to the current storage path.
pub fn save(config: &AppConfig) -> Result<(), ConfigError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut stored = config.clone();
    stored.active_cave = None;
    let yaml = serde_yml::to_string(&stored)?;
    std::fs::write(&path, yaml)?;
    Ok(())
}

/// Ensure the config directory and file exist, creating defaults if needed.
pub fn ensure() -> Result<AppConfig, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        save(&AppConfig::default())?;
    }
    load()
}

/// Ensure a cave's `.granit/` directory exists.
pub fn ensure_cave(cave_path: &Path) -> Result<(), ConfigError> {
    std::fs::create_dir_all(cave_path.join(".granit"))?;
    Ok(())
}

fn config_path() -> Result<PathBuf, ConfigError> {
    dirs::config_dir()
        .map(|d| d.join("granit").join("config.yml"))
        .ok_or(ConfigError::NoConfigDir)
}
