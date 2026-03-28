mod error;
mod secrets;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use error::ConfigError;
pub use granit_types::AgentConfig;
pub use secrets::Secrets;

/// Resolved application configuration (defaults ← global ← cave).
#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub recent_caves: Vec<PathBuf>,
    pub agent: AgentConfig,
    /// Runtime-only: the path of the currently open cave. Not persisted to YAML.
    pub active_cave: Option<PathBuf>,
}

/// Raw config as stored in YAML (all fields optional for layered merging).
#[derive(Debug, Default, Serialize, Deserialize)]
struct RawConfig {
    recent_caves: Option<Vec<PathBuf>>,
    agent: Option<RawAgentConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawAgentConfig {
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
}

impl AppConfig {
    /// Load config with layered precedence: defaults ← global ← cave.
    /// `cave_path` is None if no cave is currently open.
    ///
    /// Note: cave-level overrides affect the resolved `agent` config at open
    /// time, but are not currently exposed as an editable UI surface. The
    /// settings modal always reads and writes the global config only.
    pub fn load(cave_path: Option<&Path>) -> Result<Self, ConfigError> {
        let global = Self::load_raw(&Self::global_config_path()?)?;
        let cave = cave_path
            .map(|p| Self::load_raw(&p.join(".granit").join("config.yml")))
            .transpose()?;

        Ok(Self::merge(global, cave))
    }

    /// Persist the global config to disk.
    pub fn save_global(&self) -> Result<(), ConfigError> {
        let path = Self::global_config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let raw = RawConfig {
            recent_caves: Some(self.recent_caves.clone()),
            agent: Some(RawAgentConfig {
                provider: Some(self.agent.provider.clone()),
                model: Some(self.agent.model.clone()),
                base_url: self.agent.base_url.clone(),
            }),
        };

        let yaml = serde_yml::to_string(&raw)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Add a cave to the recent list (moves to front if already present).
    pub fn add_recent_cave(&mut self, path: PathBuf) {
        self.recent_caves.retain(|p| p != &path);
        self.recent_caves.insert(0, path);
        // Keep a reasonable limit
        self.recent_caves.truncate(10);
    }

    /// Convert to the IPC-facing type used at Tauri command boundaries.
    /// Paths are converted to strings; `active_cave` must be set separately
    /// by the caller if a cave is currently open.
    pub fn to_ipc(&self) -> granit_types::AppConfig {
        granit_types::AppConfig {
            recent_caves: self
                .recent_caves
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            agent: self.agent.clone(),
            active_cave: self
                .active_cave
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        }
    }

    /// Ensure the global config directory and default config file exist.
    /// Returns the loaded config (creating defaults if needed).
    pub fn ensure_global() -> Result<Self, ConfigError> {
        let path = Self::global_config_path()?;
        if !path.exists() {
            let config = Self {
                recent_caves: Vec::new(),
                agent: AgentConfig::default(),
                active_cave: None,
            };
            config.save_global()?;
        }
        Self::load(None)
    }

    /// Ensure a cave's `.granit/` directory and default config files exist.
    /// Creates `.granit/config.yml` with empty overrides and ensures `.gitignore` covers secrets.
    pub fn ensure_cave(cave_path: &Path) -> Result<(), ConfigError> {
        let granit_dir = cave_path.join(".granit");
        std::fs::create_dir_all(&granit_dir)?;

        let config_path = granit_dir.join("config.yml");
        if !config_path.exists() {
            let raw = RawConfig::default();
            let yaml = serde_yml::to_string(&raw)?;
            std::fs::write(&config_path, yaml)?;
        }

        ensure_cave_gitignore(cave_path)?;
        Ok(())
    }

    fn global_config_path() -> Result<PathBuf, ConfigError> {
        dirs::config_dir()
            .map(|d| d.join("granit").join("config.yml"))
            .ok_or(ConfigError::NoConfigDir)
    }

    fn load_raw(path: &Path) -> Result<RawConfig, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(serde_yml::from_str(&contents)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RawConfig::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn merge(global: RawConfig, cave: Option<RawConfig>) -> Self {
        let defaults = AppConfig {
            recent_caves: Vec::new(),
            agent: AgentConfig::default(),
            active_cave: None,
        };

        // Apply global over defaults
        let mut config = AppConfig {
            recent_caves: global.recent_caves.unwrap_or(defaults.recent_caves),
            agent: AgentConfig {
                provider: global
                    .agent
                    .as_ref()
                    .and_then(|a| a.provider.clone())
                    .unwrap_or(defaults.agent.provider),
                model: global
                    .agent
                    .as_ref()
                    .and_then(|a| a.model.clone())
                    .unwrap_or(defaults.agent.model),
                base_url: global.agent.as_ref().and_then(|a| a.base_url.clone()),
            },
            active_cave: None,
        };

        // Apply cave overrides
        if let Some(cave) = cave {
            if let Some(agent) = cave.agent {
                if let Some(provider) = agent.provider {
                    config.agent.provider = provider;
                }
                if let Some(model) = agent.model {
                    config.agent.model = model;
                }
                if let Some(base_url) = agent.base_url {
                    config.agent.base_url = Some(base_url);
                }
            }
            // recent_caves is global-only, not overridden by cave config
        }

        config
    }
}

/// Load secrets from `secrets.env` files with layered precedence.
pub fn load_secrets(cave_path: Option<&Path>) -> Result<Secrets, ConfigError> {
    let mut vars = HashMap::new();

    // Load global secrets
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("granit").join("secrets.env");
        load_env_file(&global_path, &mut vars);
    }

    // Load cave secrets (override global)
    if let Some(cave) = cave_path {
        let cave_path = cave.join(".granit").join("secrets.env");
        load_env_file(&cave_path, &mut vars);
    }

    Ok(Secrets::new(vars))
}

/// Validate that a secret value is safe for `secrets.env` storage.
/// Rejects whitespace, control characters, and non-ASCII bytes.
pub fn validate_secret_value(value: &str) -> Result<(), ConfigError> {
    if value.is_empty() {
        return Err(ConfigError::InvalidSecret("value cannot be empty".into()));
    }
    if let Some(pos) = value.find(|c: char| c.is_whitespace() || c.is_control()) {
        return Err(ConfigError::InvalidSecret(format!(
            "contains invalid character at position {pos}"
        )));
    }
    if !value.is_ascii() {
        return Err(ConfigError::InvalidSecret(
            "contains non-ASCII characters".into(),
        ));
    }
    Ok(())
}

/// Write a secret to the global `secrets.env` file.
/// Creates or updates the key in-place.
pub fn write_global_secret(key: &str, value: &str) -> Result<(), ConfigError> {
    let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    let secrets_path = config_dir.join("granit").join("secrets.env");

    // Ensure parent directory exists
    if let Some(parent) = secrets_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = std::fs::read_to_string(&secrets_path).unwrap_or_default();
    let mut found = false;
    let mut new_lines: Vec<String> = contents
        .lines()
        .map(|line| {
            if line.starts_with(&format!("{key}=")) {
                found = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        new_lines.push(format!("{key}={value}"));
    }

    let mut output = new_lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    std::fs::write(&secrets_path, output)?;
    Ok(())
}

/// Ensure the cave's `.gitignore` includes `.granit/secrets.env`.
fn ensure_cave_gitignore(cave_path: &Path) -> Result<(), ConfigError> {
    let gitignore_path = cave_path.join(".gitignore");
    let entry = ".granit/secrets.env";

    match std::fs::read_to_string(&gitignore_path) {
        Ok(contents) => {
            if !contents.lines().any(|line| line.trim() == entry) {
                let mut new_contents = contents;
                if !new_contents.ends_with('\n') {
                    new_contents.push('\n');
                }
                new_contents.push_str(entry);
                new_contents.push('\n');
                std::fs::write(&gitignore_path, new_contents)?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::write(&gitignore_path, format!("{entry}\n"))?;
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

fn load_env_file(path: &Path, vars: &mut HashMap<String, String>) {
    if let Ok(iter) = dotenvy::from_path_iter(path) {
        for item in iter.flatten() {
            vars.insert(item.0, item.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_merge_defaults_only() {
        let config = AppConfig::merge(RawConfig::default(), None);
        assert_eq!(config.agent.provider, "ollama");
        assert_eq!(config.agent.model, "qwen3.5:9b");
        assert!(config.recent_caves.is_empty());
    }

    #[test]
    fn test_merge_global_overrides_defaults() {
        let global = RawConfig {
            recent_caves: Some(vec![PathBuf::from("/notes")]),
            agent: Some(RawAgentConfig {
                provider: Some("anthropic".to_string()),
                model: None,
                base_url: None,
            }),
        };
        let config = AppConfig::merge(global, None);
        assert_eq!(config.agent.provider, "anthropic");
        assert_eq!(config.agent.model, "qwen3.5:9b"); // default preserved
        assert_eq!(config.recent_caves.len(), 1);
    }

    #[test]
    fn test_merge_cave_overrides_global() {
        let global = RawConfig {
            recent_caves: None,
            agent: Some(RawAgentConfig {
                provider: Some("openai".to_string()),
                model: Some("gpt-4o".to_string()),
                base_url: None,
            }),
        };
        let cave = RawConfig {
            recent_caves: None,
            agent: Some(RawAgentConfig {
                provider: None,
                model: Some("gpt-4o-mini".to_string()),
                base_url: None,
            }),
        };
        let config = AppConfig::merge(global, Some(cave));
        assert_eq!(config.agent.provider, "openai");
        assert_eq!(config.agent.model, "gpt-4o-mini");
    }

    #[test]
    fn test_load_raw_missing_file() {
        let raw = AppConfig::load_raw(Path::new("/nonexistent/config.yml")).unwrap();
        assert!(raw.recent_caves.is_none());
    }

    #[test]
    fn test_load_raw_valid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        fs::write(
            &path,
            "agent:\n  provider: anthropic\n  model: claude-sonnet-4-20250514\n",
        )
        .unwrap();

        let raw = AppConfig::load_raw(&path).unwrap();
        assert_eq!(
            raw.agent.as_ref().unwrap().provider.as_deref(),
            Some("anthropic")
        );
    }

    #[test]
    fn test_add_recent_cave_deduplicates() {
        let mut config = AppConfig::merge(RawConfig::default(), None);
        config.add_recent_cave(PathBuf::from("/a"));
        config.add_recent_cave(PathBuf::from("/b"));
        config.add_recent_cave(PathBuf::from("/a")); // should move to front
        assert_eq!(config.recent_caves[0], PathBuf::from("/a"));
        assert_eq!(config.recent_caves[1], PathBuf::from("/b"));
        assert_eq!(config.recent_caves.len(), 2);
    }

    #[test]
    fn test_ensure_cave_gitignore_creates_new() {
        let dir = tempfile::tempdir().unwrap();
        ensure_cave_gitignore(dir.path()).unwrap();

        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(contents.contains(".granit/secrets.env"));
    }

    #[test]
    fn test_ensure_cave_gitignore_appends() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

        ensure_cave_gitignore(dir.path()).unwrap();

        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(contents.contains("*.log"));
        assert!(contents.contains(".granit/secrets.env"));
    }

    #[test]
    fn test_ensure_cave_gitignore_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), ".granit/secrets.env\n").unwrap();

        ensure_cave_gitignore(dir.path()).unwrap();

        let contents = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(
            contents.matches(".granit/secrets.env").count(),
            1,
            "should not duplicate entry"
        );
    }

    #[test]
    fn test_secrets_from_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let granit_dir = dir.path().join(".granit");
        fs::create_dir_all(&granit_dir).unwrap();
        fs::write(
            granit_dir.join("secrets.env"),
            "AGENT_API_KEY=sk-test-123\n",
        )
        .unwrap();

        let secrets = load_secrets(Some(dir.path())).unwrap();
        assert_eq!(secrets.agent_api_key(), Some("sk-test-123"));
    }

    #[test]
    fn test_ensure_cave_creates_granit_dir_and_config() {
        let dir = tempfile::tempdir().unwrap();
        AppConfig::ensure_cave(dir.path()).unwrap();

        let granit_dir = dir.path().join(".granit");
        assert!(granit_dir.exists(), ".granit/ dir should be created");
        assert!(
            granit_dir.join("config.yml").exists(),
            "config.yml should be created"
        );

        // gitignore should also be set up
        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains(".granit/secrets.env"));
    }

    #[test]
    fn test_ensure_cave_does_not_overwrite_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let granit_dir = dir.path().join(".granit");
        fs::create_dir_all(&granit_dir).unwrap();
        fs::write(
            granit_dir.join("config.yml"),
            "agent:\n  model: custom-model\n",
        )
        .unwrap();

        AppConfig::ensure_cave(dir.path()).unwrap();

        let contents = fs::read_to_string(granit_dir.join("config.yml")).unwrap();
        assert!(
            contents.contains("custom-model"),
            "should not overwrite existing config"
        );
    }

    #[test]
    fn test_save_and_reload_global() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yml");

        let config = AppConfig {
            recent_caves: vec![PathBuf::from("/my/notes")],
            agent: AgentConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                base_url: None,
            },
            active_cave: None,
        };

        // Save manually to temp path (bypassing global_config_path)
        let raw = RawConfig {
            recent_caves: Some(config.recent_caves.clone()),
            agent: Some(RawAgentConfig {
                provider: Some(config.agent.provider.clone()),
                model: Some(config.agent.model.clone()),
                base_url: config.agent.base_url.clone(),
            }),
        };
        let yaml = serde_yml::to_string(&raw).unwrap();
        fs::write(&config_path, yaml).unwrap();

        // Reload and verify
        let loaded = AppConfig::load_raw(&config_path).unwrap();
        assert_eq!(
            loaded.agent.as_ref().unwrap().provider.as_deref(),
            Some("anthropic")
        );
        assert_eq!(loaded.recent_caves.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_validate_secret_value_accepts_valid_key() {
        assert!(validate_secret_value("sk-ant-api03-abc123_DEF").is_ok());
    }

    #[test]
    fn test_validate_secret_value_rejects_empty() {
        assert!(validate_secret_value("").is_err());
    }

    #[test]
    fn test_validate_secret_value_rejects_spaces() {
        assert!(validate_secret_value("sk-ant abc").is_err());
    }

    #[test]
    fn test_validate_secret_value_rejects_newlines() {
        assert!(validate_secret_value("sk-ant\nabc").is_err());
    }

    #[test]
    fn test_validate_secret_value_rejects_non_ascii() {
        assert!(validate_secret_value("sk-ant-│abc").is_err());
    }
}
