mod error;
mod secrets;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use error::ConfigError;
pub use granit_types::{AgentConfig, FontConfig};
pub use secrets::Secrets;

/// Apply optional raw config fields over a resolved config value.
trait MergeRaw<R> {
    fn merge_raw(&mut self, raw: R);
}

impl MergeRaw<RawAgentConfig> for AgentConfig {
    fn merge_raw(&mut self, raw: RawAgentConfig) {
        if let Some(provider) = raw.provider {
            self.provider = provider;
        }
        if let Some(model) = raw.model {
            self.model = model;
        }
        if let Some(base_url) = raw.base_url {
            self.base_url = Some(base_url);
        }
        if let Some(max_history) = raw.max_history {
            self.max_history = max_history;
        }
    }
}

impl MergeRaw<RawFontConfig> for FontConfig {
    fn merge_raw(&mut self, raw: RawFontConfig) {
        if let Some(family) = raw.font_family {
            self.font_family = family;
        }
        if let Some(size) = raw.font_size {
            self.font_size = size;
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub recent_caves: Vec<PathBuf>,
    pub agent: AgentConfig,
    pub markdown_font: FontConfig,
    pub reading_font: FontConfig,
    pub agent_font: FontConfig,
    /// Runtime-only: the path of the currently open cave. Not persisted to YAML.
    pub active_cave: Option<PathBuf>,
}

/// Raw config as stored in YAML (all fields optional for layered merging).
#[derive(Debug, Default, Serialize, Deserialize)]
struct RawConfig {
    recent_caves: Option<Vec<PathBuf>>,
    agent: Option<RawAgentConfig>,
    markdown_font: Option<RawFontConfig>,
    reading_font: Option<RawFontConfig>,
    agent_font: Option<RawFontConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawAgentConfig {
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    max_history: Option<usize>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawFontConfig {
    font_family: Option<String>,
    font_size: Option<u8>,
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
                max_history: Some(self.agent.max_history),
            }),
            markdown_font: Some(RawFontConfig {
                font_family: Some(self.markdown_font.font_family.clone()),
                font_size: Some(self.markdown_font.font_size),
            }),
            reading_font: Some(RawFontConfig {
                font_family: Some(self.reading_font.font_family.clone()),
                font_size: Some(self.reading_font.font_size),
            }),
            agent_font: Some(RawFontConfig {
                font_family: Some(self.agent_font.font_family.clone()),
                font_size: Some(self.agent_font.font_size),
            }),
        };

        let yaml = serde_yml::to_string(&raw)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Add a cave to the recent list (moves to front if already present).
    #[cfg(test)]
    pub fn add_recent_cave(&mut self, path: PathBuf) {
        self.recent_caves.retain(|p| p != &path);
        self.recent_caves.insert(0, path);
        // Keep a reasonable limit
        self.recent_caves.truncate(10);
    }

    /// Update only the `recent_caves` list in the global config file.
    ///
    /// Unlike `save_global()`, this reads the raw global config, patches
    /// just the recent-caves field, and writes it back — so cave-layer
    /// overrides that live in the in-memory merged config are never
    /// written to the global file.
    pub fn save_recent_cave(path: &Path) -> Result<(), ConfigError> {
        let config_path = Self::global_config_path()?;
        let mut raw = Self::load_raw(&config_path)?;
        let mut caves = raw.recent_caves.unwrap_or_default();
        caves.retain(|p| p != path);
        caves.insert(0, path.to_path_buf());
        caves.truncate(10);
        raw.recent_caves = Some(caves);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yml::to_string(&raw)?;
        std::fs::write(&config_path, yaml)?;
        Ok(())
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
            markdown_font: self.markdown_font.clone(),
            reading_font: self.reading_font.clone(),
            agent_font: self.agent_font.clone(),
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
                markdown_font: FontConfig::markdown_default(),
                reading_font: FontConfig::reading_default(),
                agent_font: FontConfig::agent_default(),
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

    fn merge(mut global: RawConfig, cave: Option<RawConfig>) -> Self {
        // Extract recent_caves before applying as raw overrides
        let recent_caves = global.recent_caves.take().unwrap_or_default();

        let mut config = AppConfig {
            recent_caves,
            agent: AgentConfig::default(),
            markdown_font: FontConfig::markdown_default(),
            reading_font: FontConfig::reading_default(),
            agent_font: FontConfig::agent_default(),
            active_cave: None,
        };

        Self::apply_raw_overrides(&mut config, global);
        if let Some(cave) = cave {
            Self::apply_raw_overrides(&mut config, cave);
        }

        config
    }

    fn apply_raw_overrides(config: &mut AppConfig, raw: RawConfig) {
        // recent_caves is global-only, not overridden by cave config
        if let Some(agent) = raw.agent {
            config.agent.merge_raw(agent);
        }
        if let Some(font) = raw.markdown_font {
            config.markdown_font.merge_raw(font);
        }
        if let Some(font) = raw.reading_font {
            config.reading_font.merge_raw(font);
        }
        if let Some(font) = raw.agent_font {
            config.agent_font.merge_raw(font);
        }
    }
}

/// Load secrets from `secrets.env` files with layered precedence.
pub fn load_secrets(cave_path: Option<&Path>) -> Result<Secrets, ConfigError> {
    let mut vars = HashMap::new();

    // Load global secrets
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("granit").join("secrets.env");
        load_env_file(&global_path, &mut vars)?;
    }

    // Load cave secrets (override global)
    if let Some(cave) = cave_path {
        let cave_path = cave.join(".granit").join("secrets.env");
        load_env_file(&cave_path, &mut vars)?;
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

fn load_env_file(path: &Path, vars: &mut HashMap<String, String>) -> Result<(), ConfigError> {
    let iter = match dotenvy::from_path_iter(path) {
        Ok(iter) => iter,
        Err(e) if e.not_found() => return Ok(()),
        Err(e) => {
            return Err(ConfigError::EnvFile {
                path: path.display().to_string(),
                reason: e.to_string(),
            })
        }
    };
    for item in iter {
        match item {
            Ok((key, value)) => {
                vars.insert(key, value);
            }
            Err(e) => {
                return Err(ConfigError::EnvFile {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })
            }
        }
    }
    Ok(())
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
                max_history: None,
            }),
            ..Default::default()
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
                max_history: None,
            }),
            ..Default::default()
        };
        let cave = RawConfig {
            recent_caves: None,
            agent: Some(RawAgentConfig {
                provider: None,
                model: Some("gpt-4o-mini".to_string()),
                base_url: None,
                max_history: None,
            }),
            ..Default::default()
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
                ..AgentConfig::default()
            },
            markdown_font: FontConfig::markdown_default(),
            reading_font: FontConfig::reading_default(),
            agent_font: FontConfig::agent_default(),
            active_cave: None,
        };

        // Save manually to temp path (bypassing global_config_path)
        let raw = RawConfig {
            recent_caves: Some(config.recent_caves.clone()),
            agent: Some(RawAgentConfig {
                provider: Some(config.agent.provider.clone()),
                model: Some(config.agent.model.clone()),
                base_url: config.agent.base_url.clone(),
                max_history: None,
            }),
            ..Default::default()
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

    #[test]
    fn test_merge_defaults_includes_font_defaults() {
        let config = AppConfig::merge(RawConfig::default(), None);
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
        assert_eq!(config.reading_font, FontConfig::reading_default());
        assert_eq!(config.agent_font, FontConfig::agent_default());
    }

    #[test]
    fn test_merge_font_partial_override() {
        let global = RawConfig {
            markdown_font: Some(RawFontConfig {
                font_family: Some("JetBrains Mono".to_string()),
                font_size: None,
            }),
            ..Default::default()
        };
        let config = AppConfig::merge(global, None);
        assert_eq!(config.markdown_font.font_family, "JetBrains Mono");
        assert_eq!(config.markdown_font.font_size, 14); // default preserved
    }

    #[test]
    fn test_merge_font_cave_overrides_global() {
        let global = RawConfig {
            markdown_font: Some(RawFontConfig {
                font_family: Some("monospace".to_string()),
                font_size: Some(14),
            }),
            ..Default::default()
        };
        let cave = RawConfig {
            markdown_font: Some(RawFontConfig {
                font_family: None,
                font_size: Some(18),
            }),
            ..Default::default()
        };
        let config = AppConfig::merge(global, Some(cave));
        assert_eq!(config.markdown_font.font_family, "monospace");
        assert_eq!(config.markdown_font.font_size, 18);
    }

    #[test]
    fn test_load_yaml_without_font_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        fs::write(&path, "agent:\n  provider: ollama\n").unwrap();

        let raw = AppConfig::load_raw(&path).unwrap();
        assert!(raw.markdown_font.is_none());
        assert!(raw.reading_font.is_none());
        assert!(raw.agent_font.is_none());

        // Merge should fill in defaults
        let config = AppConfig::merge(raw, None);
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
    }
}
