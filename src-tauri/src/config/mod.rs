mod error;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use error::ConfigError;
pub use granit_types::{AgentConfig, FontConfig, ProviderEntry, SidebarConfig};

/// Apply optional raw config fields over a resolved config value.
trait MergeRaw<R> {
    fn merge_raw(&mut self, raw: R);
}

impl MergeRaw<RawAgentConfig> for AgentConfig {
    fn merge_raw(&mut self, raw: RawAgentConfig) {
        if let Some(providers) = raw.providers {
            self.providers = providers;
        }
        if let Some(selected_provider) = raw.selected_provider {
            self.selected_provider = selected_provider;
        }
        if let Some(selected_model) = raw.selected_model {
            self.selected_model = Some(selected_model);
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

impl MergeRaw<RawSidebarConfig> for SidebarConfig {
    fn merge_raw(&mut self, raw: RawSidebarConfig) {
        if let Some(visible) = raw.visible {
            self.visible = visible;
        }
        if let Some(width) = raw.width {
            self.width = width;
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
    pub sidebar: SidebarConfig,
    pub agent_panel: SidebarConfig,
    /// Active theme id — any DaisyUI theme name or custom theme (e.g. "dark", "catppuccin-mocha").
    pub theme: String,
    /// Folder name/path (relative to cave root) where daily notes are stored.
    pub daily_note_folder: String,
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
    sidebar: Option<RawSidebarConfig>,
    agent_panel: Option<RawSidebarConfig>,
    theme: Option<String>,
    daily_note_folder: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawAgentConfig {
    providers: Option<Vec<ProviderEntry>>,
    selected_provider: Option<usize>,
    selected_model: Option<String>,
    max_history: Option<usize>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawFontConfig {
    font_family: Option<String>,
    font_size: Option<u8>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawSidebarConfig {
    visible: Option<bool>,
    width: Option<u16>,
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
                providers: Some(self.agent.providers.clone()),
                selected_provider: Some(self.agent.selected_provider),
                selected_model: self.agent.selected_model.clone(),
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
            sidebar: Some(RawSidebarConfig {
                visible: Some(self.sidebar.visible),
                width: Some(self.sidebar.width),
            }),
            agent_panel: Some(RawSidebarConfig {
                visible: Some(self.agent_panel.visible),
                width: Some(self.agent_panel.width),
            }),
            theme: Some(self.theme.clone()),
            daily_note_folder: Some(self.daily_note_folder.clone()),
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
            sidebar: self.sidebar.clone(),
            agent_panel: self.agent_panel.clone(),
            theme: self.theme.clone(),
            daily_note_folder: self.daily_note_folder.clone(),
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
                sidebar: SidebarConfig::sidebar_default(),
                agent_panel: SidebarConfig::agent_default(),
                theme: "dark".to_string(),
                daily_note_folder: "Daily".to_string(),
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
            sidebar: SidebarConfig::sidebar_default(),
            agent_panel: SidebarConfig::agent_default(),
            theme: "default".to_string(),
            daily_note_folder: "Daily".to_string(),
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
        if let Some(sb) = raw.sidebar {
            config.sidebar.merge_raw(sb);
        }
        if let Some(ap) = raw.agent_panel {
            config.agent_panel.merge_raw(ap);
        }
        if let Some(folder) = raw.daily_note_folder {
            config.daily_note_folder = folder;
        }
        if let Some(theme) = raw.theme {
            config.theme = theme;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::ProviderConfig;
    use std::fs;

    #[test]
    fn test_merge_defaults_only() {
        let config = AppConfig::merge(RawConfig::default(), None);
        assert_eq!(config.agent.providers.len(), 1);
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Ollama { .. }
        ));
        assert!(config.recent_caves.is_empty());
    }

    #[test]
    fn test_merge_global_overrides_defaults() {
        let global = RawConfig {
            recent_caves: Some(vec![PathBuf::from("/notes")]),
            agent: Some(RawAgentConfig {
                providers: Some(vec![ProviderEntry {
                    name: None,
                    provider: ProviderConfig::Anthropic {
                        api_key: "sk-test".into(),
                    },
                }]),
                selected_provider: None,
                selected_model: None,
                max_history: None,
            }),
            ..Default::default()
        };
        let config = AppConfig::merge(global, None);
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
        assert_eq!(config.agent.max_history, 100); // default preserved
        assert_eq!(config.recent_caves.len(), 1);
    }

    #[test]
    fn test_merge_cave_overrides_global_providers() {
        let global = RawConfig {
            recent_caves: None,
            agent: Some(RawAgentConfig {
                providers: Some(vec![ProviderEntry {
                    name: None,
                    provider: ProviderConfig::Ollama { base_url: None },
                }]),
                selected_provider: Some(0),
                selected_model: Some("global-model".into()),
                max_history: None,
            }),
            ..Default::default()
        };
        let cave = RawConfig {
            recent_caves: None,
            agent: Some(RawAgentConfig {
                providers: Some(vec![ProviderEntry {
                    name: None,
                    provider: ProviderConfig::Anthropic {
                        api_key: "cave-key".into(),
                    },
                }]),
                selected_provider: None,
                selected_model: Some("cave-model".into()),
                max_history: None,
            }),
            ..Default::default()
        };
        let config = AppConfig::merge(global, Some(cave));
        // Cave providers replace global entirely
        assert_eq!(config.agent.providers.len(), 1);
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
        assert_eq!(config.agent.selected_model.as_deref(), Some("cave-model"));
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
            "agent:\n  providers:\n    - provider: anthropic\n      api_key: sk-test\n",
        )
        .unwrap();

        let raw = AppConfig::load_raw(&path).unwrap();
        let providers = raw.agent.as_ref().unwrap().providers.as_ref().unwrap();
        assert!(matches!(
            providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
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
    fn test_ensure_cave_creates_granit_dir_and_config() {
        let dir = tempfile::tempdir().unwrap();
        AppConfig::ensure_cave(dir.path()).unwrap();

        let granit_dir = dir.path().join(".granit");
        assert!(granit_dir.exists(), ".granit/ dir should be created");
        assert!(
            granit_dir.join("config.yml").exists(),
            "config.yml should be created"
        );
    }

    #[test]
    fn test_ensure_cave_does_not_overwrite_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let granit_dir = dir.path().join(".granit");
        fs::create_dir_all(&granit_dir).unwrap();
        fs::write(
            granit_dir.join("config.yml"),
            "agent:\n  selected_model: custom-model\n",
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
                providers: vec![ProviderEntry {
                    name: None,
                    provider: ProviderConfig::Anthropic {
                        api_key: "sk-ant-test".into(),
                    },
                }],
                selected_provider: 0,
                selected_model: Some("claude-sonnet-4-20250514".into()),
                ..AgentConfig::default()
            },
            markdown_font: FontConfig::markdown_default(),
            reading_font: FontConfig::reading_default(),
            agent_font: FontConfig::agent_default(),
            sidebar: SidebarConfig::sidebar_default(),
            agent_panel: SidebarConfig::agent_default(),
            theme: "default".to_string(),
            daily_note_folder: "Daily".to_string(),
            active_cave: None,
        };

        // Save manually to temp path (bypassing global_config_path)
        let raw = RawConfig {
            recent_caves: Some(config.recent_caves.clone()),
            agent: Some(RawAgentConfig {
                providers: Some(config.agent.providers.clone()),
                selected_provider: Some(config.agent.selected_provider),
                selected_model: config.agent.selected_model.clone(),
                max_history: None,
            }),
            ..Default::default()
        };
        let yaml = serde_yml::to_string(&raw).unwrap();
        fs::write(&config_path, yaml).unwrap();

        // Reload and verify
        let loaded = AppConfig::load_raw(&config_path).unwrap();
        let providers = loaded.agent.as_ref().unwrap().providers.as_ref().unwrap();
        assert!(matches!(
            providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
        assert_eq!(loaded.recent_caves.as_ref().unwrap().len(), 1);
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
    fn test_merge_daily_note_folder_cave_overrides_global() {
        let global = RawConfig {
            daily_note_folder: Some("Journal".to_string()),
            ..Default::default()
        };
        let cave = RawConfig {
            daily_note_folder: Some("Diary".to_string()),
            ..Default::default()
        };
        let config = AppConfig::merge(global, Some(cave));
        assert_eq!(config.daily_note_folder, "Diary");
    }

    #[test]
    fn test_merge_daily_note_folder_default() {
        let config = AppConfig::merge(RawConfig::default(), None);
        assert_eq!(config.daily_note_folder, "Daily");
    }

    #[test]
    fn test_load_yaml_without_font_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        fs::write(&path, "agent:\n  selected_provider: 0\n").unwrap();

        let raw = AppConfig::load_raw(&path).unwrap();
        assert!(raw.markdown_font.is_none());
        assert!(raw.reading_font.is_none());
        assert!(raw.agent_font.is_none());

        // Merge should fill in defaults
        let config = AppConfig::merge(raw, None);
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
    }
}
