use crate::AgentConfig;
use serde::{Deserialize, Serialize};

fn default_theme() -> String {
    "dark".to_string()
}

fn default_daily_note_folder() -> String {
    "Daily".to_string()
}

/// Sidebar panel state (visibility + width).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidebarConfig {
    pub visible: bool,
    pub width: u16,
}

impl SidebarConfig {
    pub fn sidebar_default() -> Self {
        Self {
            visible: true,
            width: 256,
        }
    }

    pub fn agent_default() -> Self {
        Self {
            visible: false,
            width: 320,
        }
    }
}

/// Font configuration for a UI area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontConfig {
    pub font_family: String,
    pub font_size: u8,
}

impl FontConfig {
    pub fn markdown_default() -> Self {
        Self {
            font_family: "monospace".to_string(),
            font_size: 14,
        }
    }

    pub fn reading_default() -> Self {
        Self {
            font_family: "sans-serif".to_string(),
            font_size: 16,
        }
    }

    pub fn agent_default() -> Self {
        Self {
            font_family: "sans-serif".to_string(),
            font_size: 14,
        }
    }
}

/// Application configuration shared between backend persistence and frontend IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default = "FontConfig::markdown_default")]
    pub markdown_font: FontConfig,
    #[serde(default = "FontConfig::reading_default")]
    pub reading_font: FontConfig,
    #[serde(default = "FontConfig::agent_default")]
    pub agent_font: FontConfig,
    #[serde(default = "SidebarConfig::sidebar_default")]
    pub sidebar: SidebarConfig,
    #[serde(default = "SidebarConfig::agent_default")]
    pub agent_panel: SidebarConfig,
    /// Active theme id (e.g. "default", "mocha").
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Folder name/path (relative to cave root) where daily notes are stored.
    #[serde(default = "default_daily_note_folder")]
    pub daily_note_folder: String,
    /// Optional template slug used when creating a new daily note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_note_template_slug: Option<String>,
    /// The currently open cave path, if any. Runtime-only in backend persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_cave: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            markdown_font: FontConfig::markdown_default(),
            reading_font: FontConfig::reading_default(),
            agent_font: FontConfig::agent_default(),
            sidebar: SidebarConfig::sidebar_default(),
            agent_panel: SidebarConfig::agent_default(),
            theme: "dark".to_string(),
            daily_note_folder: "Daily".to_string(),
            daily_note_template_slug: None,
            active_cave: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderConfig;

    #[test]
    fn test_default_has_ollama_provider() {
        let config = AppConfig::default();
        assert_eq!(config.agent.providers.len(), 1);
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Ollama { .. }
        ));
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
        assert!(config.daily_note_template_slug.is_none());
    }

    #[test]
    fn test_empty_yaml_uses_all_defaults() {
        // serde_yml 0.0.13 rejects fully empty documents, so an empty mapping
        // is the minimal input; empty-file handling lives in Cave::load_config.
        let config: AppConfig = serde_yml::from_str("{}").unwrap();
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
        assert!(config.daily_note_template_slug.is_none());
        assert!(config.active_cave.is_none());
    }

    #[test]
    fn test_partial_yaml_overrides_only_specified_fields() {
        let yaml = "theme: catppuccin-mocha\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.theme, "catppuccin-mocha");
        assert_eq!(config.daily_note_folder, "Daily");
        assert!(config.daily_note_template_slug.is_none());
        assert_eq!(config.agent.max_history, 100);
    }

    #[test]
    fn test_legacy_recent_caves_field_is_ignored() {
        let yaml = "recent_caves:\n  - /old/cave\ntheme: latte\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.theme, "latte");
        assert_eq!(config.daily_note_folder, "Daily");
        assert!(config.daily_note_template_slug.is_none());
        assert!(config.active_cave.is_none());
    }

    #[test]
    fn test_yaml_with_provider_deserializes() {
        let yaml = "agent:\n  providers:\n    - provider: anthropic\n      api_key: sk-test\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert!(matches!(
            config.agent.providers[0].provider,
            ProviderConfig::Anthropic { .. }
        ));
        assert_eq!(config.agent.max_history, 100);
    }

    #[test]
    fn test_yaml_without_font_keys_uses_defaults() {
        let yaml = "agent:\n  selected_provider: 0\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.markdown_font, FontConfig::markdown_default());
        assert_eq!(config.reading_font, FontConfig::reading_default());
        assert_eq!(config.agent_font, FontConfig::agent_default());
    }

    #[test]
    fn test_active_cave_none_is_not_serialized() {
        let yaml = serde_yml::to_string(&AppConfig::default()).unwrap();
        assert!(!yaml.contains("active_cave"));
    }

    #[test]
    fn test_daily_note_template_slug_round_trips() {
        let yaml = "daily_note_template_slug: daily-template\n";
        let config: AppConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(
            config.daily_note_template_slug.as_deref(),
            Some("daily-template")
        );

        let serialized = serde_yml::to_string(&config).unwrap();
        assert!(serialized.contains("daily_note_template_slug: daily-template"));
    }
}
