mod error;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use error::ConfigError;
pub use granit_types::{AgentConfig, FontConfig, SidebarConfig};

// Default-value functions used by serde field attributes.
fn default_theme() -> String {
    "dark".to_string()
}
fn default_daily_note_folder() -> String {
    "Daily".to_string()
}
fn default_markdown_font() -> FontConfig {
    FontConfig::markdown_default()
}
fn default_reading_font() -> FontConfig {
    FontConfig::reading_default()
}
fn default_agent_font() -> FontConfig {
    FontConfig::agent_default()
}
fn default_sidebar() -> SidebarConfig {
    SidebarConfig::sidebar_default()
}
fn default_agent_panel() -> SidebarConfig {
    SidebarConfig::agent_default()
}

/// Application configuration. Serialized directly to/from `~/.config/granit/config.yml`.
/// Missing fields on load fall back to their per-field serde defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub recent_caves: Vec<PathBuf>,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default = "default_markdown_font")]
    pub markdown_font: FontConfig,
    #[serde(default = "default_reading_font")]
    pub reading_font: FontConfig,
    #[serde(default = "default_agent_font")]
    pub agent_font: FontConfig,
    #[serde(default = "default_sidebar")]
    pub sidebar: SidebarConfig,
    #[serde(default = "default_agent_panel")]
    pub agent_panel: SidebarConfig,
    /// Active theme id — any DaisyUI theme name (e.g. "dark", "catppuccin-mocha").
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Folder name (relative to cave root) where daily notes are stored.
    #[serde(default = "default_daily_note_folder")]
    pub daily_note_folder: String,
    /// Runtime-only: the path of the currently open cave. Not persisted to YAML.
    #[serde(skip)]
    pub active_cave: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            recent_caves: Vec::new(),
            agent: AgentConfig::default(),
            markdown_font: default_markdown_font(),
            reading_font: default_reading_font(),
            agent_font: default_agent_font(),
            sidebar: default_sidebar(),
            agent_panel: default_agent_panel(),
            theme: default_theme(),
            daily_note_folder: default_daily_note_folder(),
            active_cave: None,
        }
    }
}

impl AppConfig {
    /// Load config from `~/.config/granit/config.yml`.
    /// Returns defaults if the file does not exist; missing fields fall back to defaults.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        match std::fs::read_to_string(&path) {
            Ok(contents) => Ok(serde_yml::from_str(&contents)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Persist this config to `~/.config/granit/config.yml`.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yml::to_string(self)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Add a cave to the front of the recent-caves list, deduplicating and capping at 10.
    pub fn add_recent_cave(&mut self, path: PathBuf) {
        self.recent_caves.retain(|p| p != &path);
        self.recent_caves.insert(0, path);
        self.recent_caves.truncate(10);
    }

    /// Persist an updated `recent_caves` list without touching any other field.
    /// Loads the config from disk, updates the list, then saves it back.
    pub fn save_recent_cave(path: &Path) -> Result<(), ConfigError> {
        let mut config = Self::load()?;
        config.add_recent_cave(path.to_path_buf());
        config.save()
    }

    /// Convert to the IPC-facing type used at Tauri command boundaries.
    /// Paths are converted to strings; `active_cave` must be set separately by the caller.
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

    /// Ensure the config directory and file exist, creating defaults if needed.
    pub fn ensure() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        if !path.exists() {
            Self::default().save()?;
        }
        Self::load()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::ProviderConfig;
    use std::fs;

    #[test]
    fn test_default_has_ollama_provider() {
        let config = AppConfig::default();
        assert_eq!(config.agent.providers.len(), 1);
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Ollama { .. }
        ));
        assert!(config.recent_caves.is_empty());
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
    }

    #[test]
    fn test_empty_yaml_uses_all_defaults() {
        let config: AppConfig = serde_yml::from_str("").unwrap();
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
        assert!(config.recent_caves.is_empty());
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
    }

    #[test]
    fn test_partial_yaml_overrides_only_specified_fields() {
        let yaml = "theme: catppuccin-mocha\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.theme, "catppuccin-mocha");
        assert_eq!(config.daily_note_folder, "Daily"); // default preserved
        assert_eq!(config.agent.max_history, 100); // AgentConfig default preserved
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");

        let mut config = AppConfig::default();
        config.theme = "light".to_string();
        config.recent_caves = vec![PathBuf::from("/my/notes")];
        config.active_cave = Some(PathBuf::from("/should/not/persist"));

        let yaml = serde_yml::to_string(&config).unwrap();
        fs::write(&path, &yaml).unwrap();

        let loaded: AppConfig = serde_yml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.recent_caves, vec![PathBuf::from("/my/notes")]);
        assert_eq!(loaded.daily_note_folder, "Daily");
        assert!(loaded.active_cave.is_none()); // #[serde(skip)]
    }

    #[test]
    fn test_active_cave_not_serialized() {
        let mut config = AppConfig::default();
        config.active_cave = Some(PathBuf::from("/secret/cave"));
        let yaml = serde_yml::to_string(&config).unwrap();
        assert!(
            !yaml.contains("active_cave"),
            "active_cave must not appear in YAML"
        );
        assert!(
            !yaml.contains("secret"),
            "cave path must not appear in YAML"
        );
    }

    #[test]
    fn test_add_recent_cave_deduplicates_and_moves_to_front() {
        let mut config = AppConfig::default();
        config.add_recent_cave(PathBuf::from("/a"));
        config.add_recent_cave(PathBuf::from("/b"));
        config.add_recent_cave(PathBuf::from("/a")); // should move to front
        assert_eq!(config.recent_caves[0], PathBuf::from("/a"));
        assert_eq!(config.recent_caves.len(), 2);
    }

    #[test]
    fn test_add_recent_cave_caps_at_10() {
        let mut config = AppConfig::default();
        for i in 0..=10 {
            config.add_recent_cave(PathBuf::from(format!("/{i}")));
        }
        assert_eq!(config.recent_caves.len(), 10);
    }

    #[test]
    fn test_yaml_with_provider_deserializes() {
        let yaml = "agent:\n  providers:\n    - provider: anthropic\n      api_key: sk-test\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
        assert_eq!(config.agent.max_history, 100); // default preserved
    }

    #[test]
    fn test_yaml_without_font_keys_uses_defaults() {
        let yaml = "agent:\n  selected_provider: 0\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
        assert_eq!(config.reading_font, FontConfig::reading_default());
        assert_eq!(config.agent_font, FontConfig::agent_default());
    }

    // ── to_ipc ─────────────────────────────────────────────────────────────

    #[test]
    fn test_to_ipc_recent_caves_as_strings() {
        let mut config = AppConfig::default();
        config.recent_caves = vec![
            PathBuf::from("/home/user/notes"),
            PathBuf::from("/tmp/cave"),
        ];
        let ipc = config.to_ipc();
        assert_eq!(
            ipc.recent_caves,
            vec!["/home/user/notes".to_string(), "/tmp/cave".to_string()]
        );
    }

    #[test]
    fn test_to_ipc_active_cave_propagated() {
        let mut config = AppConfig::default();
        config.active_cave = Some(PathBuf::from("/active/cave"));
        let ipc = config.to_ipc();
        assert_eq!(ipc.active_cave, Some("/active/cave".to_string()));
    }

    #[test]
    fn test_to_ipc_active_cave_none_when_unset() {
        let config = AppConfig::default();
        let ipc = config.to_ipc();
        assert!(ipc.active_cave.is_none());
        assert!(ipc.recent_caves.is_empty());
    }

    #[test]
    fn test_to_ipc_preserves_theme_and_fonts() {
        let mut config = AppConfig::default();
        config.theme = "cupcake".to_string();
        let ipc = config.to_ipc();
        assert_eq!(ipc.theme, "cupcake");
        assert_eq!(ipc.markdown_font, FontConfig::markdown_default());
    }
}
