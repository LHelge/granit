use crate::AgentConfig;
use serde::{Deserialize, Serialize};

fn default_theme() -> String {
    ThemeChoice::System.config_value().to_string()
}

/// The theme the GPUI app shows: Catppuccin Latte (light), Catppuccin Frappé
/// (dark), or whichever matches the system appearance.
///
/// It is read from [`AppConfig::theme`], which keeps the free-form name the
/// Tauri app writes (a DaisyUI or community theme id). Reading never fails
/// and never changes the stored name: known dark themes read as
/// [`ThemeChoice::Dark`], known light ones as [`ThemeChoice::Light`], and a
/// name that says neither as [`ThemeChoice::System`]. Choosing a theme writes
/// [`ThemeChoice::config_value`], which the Tauri app also accepts: DaisyUI's
/// built-in `light` and `dark`, and `system`, which no theme is named after,
/// so the web view falls back to DaisyUI's `prefers-color-scheme` themes and
/// follows the system too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follow the system appearance.
    #[default]
    System,
    Light,
    Dark,
}

/// DaisyUI and community theme ids the Tauri app offers with a dark palette.
const DARK_THEMES: &[&str] = &[
    "dark",
    "synthwave",
    "halloween",
    "forest",
    "black",
    "luxury",
    "dracula",
    "business",
    "night",
    "coffee",
    "dim",
    "sunset",
    "abyss",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "frappe",
    "macchiato",
    "mocha",
    "gruvbox-dark",
    "tokyo-night",
    "rose-pine-moon",
    "one-dark",
];

/// DaisyUI and community theme ids the Tauri app offers with a light palette.
const LIGHT_THEMES: &[&str] = &[
    "light",
    "cupcake",
    "bumblebee",
    "emerald",
    "corporate",
    "retro",
    "cyberpunk",
    "valentine",
    "garden",
    "aqua",
    "lofi",
    "pastel",
    "fantasy",
    "wireframe",
    "cmyk",
    "autumn",
    "acid",
    "lemonade",
    "winter",
    "nord",
    "caramellatte",
    "silk",
    "catppuccin-latte",
    "latte",
    "gruvbox-light",
    "rose-pine-dawn",
];

impl ThemeChoice {
    /// Every choice, in the order the settings offer them.
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// Read a stored theme name. Never fails: an unknown name follows the
    /// system unless it says "dark" or "light".
    pub fn from_config(theme: &str) -> Self {
        let theme = theme.trim().to_ascii_lowercase();
        if DARK_THEMES.contains(&theme.as_str()) {
            Self::Dark
        } else if LIGHT_THEMES.contains(&theme.as_str()) {
            Self::Light
        } else if theme.contains("dark") {
            Self::Dark
        } else if theme.contains("light") {
            Self::Light
        } else {
            Self::System
        }
    }

    /// The name stored for this choice, one the Tauri app also accepts.
    pub fn config_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// What the settings call this choice.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "Follow the system",
            Self::Light => "Catppuccin Latte (light)",
            Self::Dark => "Catppuccin Frappé (dark)",
        }
    }
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
            width: 320,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Active theme id: a DaisyUI or community theme id from the Tauri app
    /// (e.g. "dracula", "catppuccin-mocha"), or "system". The GPUI app reads
    /// it through [`AppConfig::theme_choice`].
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Folder name/path (relative to cave root) where daily notes are stored.
    #[serde(default = "default_daily_note_folder")]
    pub daily_note_folder: String,
    /// Optional template slug used when creating a new daily note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_note_template_slug: Option<String>,
    /// Cloud backup settings (backend URL and API key).
    #[serde(default)]
    pub backup: crate::BackupConfig,
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
            theme: default_theme(),
            daily_note_folder: "Daily".to_string(),
            daily_note_template_slug: None,
            backup: crate::BackupConfig::default(),
            active_cave: None,
        }
    }
}

impl AppConfig {
    /// The theme to show, read from [`AppConfig::theme`].
    pub fn theme_choice(&self) -> ThemeChoice {
        ThemeChoice::from_config(&self.theme)
    }

    /// Choose `choice`. A choice equal to what the stored name reads as
    /// keeps that name, so an unchanged pick never rewrites a Tauri theme.
    pub fn set_theme_choice(&mut self, choice: ThemeChoice) {
        if self.theme_choice() != choice {
            self.theme = choice.config_value().to_string();
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
        assert_eq!(config.theme, "system");
        assert_eq!(config.theme_choice(), ThemeChoice::System);
        assert_eq!(config.daily_note_folder, "Daily");
        assert!(config.daily_note_template_slug.is_none());
    }

    #[test]
    fn test_empty_yaml_uses_all_defaults() {
        // serde_yml 0.0.13 rejects fully empty documents, so an empty mapping
        // is the minimal input; empty-file handling lives in Cave::load_config.
        let config: AppConfig = serde_yml::from_str("{}").unwrap();
        assert_eq!(config.theme, "system");
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
    fn test_daisyui_theme_names_map_to_the_nearest_catppuccin_flavour() {
        for dark in [
            "dark",
            "dracula",
            "catppuccin-mocha",
            "tokyo-night",
            "my-dark-theme",
        ] {
            assert_eq!(ThemeChoice::from_config(dark), ThemeChoice::Dark, "{dark}");
        }
        for light in ["light", "cupcake", "catppuccin-latte", "latte", "Nord"] {
            assert_eq!(
                ThemeChoice::from_config(light),
                ThemeChoice::Light,
                "{light}"
            );
        }
        for system in ["system", "default", "", "no-such-theme"] {
            assert_eq!(
                ThemeChoice::from_config(system),
                ThemeChoice::System,
                "{system}"
            );
        }
    }

    #[test]
    fn test_reading_a_daisyui_theme_keeps_its_name() {
        let config: AppConfig = serde_yml::from_str("theme: dracula\n").unwrap();
        assert_eq!(config.theme_choice(), ThemeChoice::Dark);
        let yaml = serde_yml::to_string(&config).unwrap();
        assert!(yaml.contains("theme: dracula"), "{yaml}");
    }

    #[test]
    fn test_choosing_a_theme_writes_a_name_the_tauri_app_accepts() {
        let mut config: AppConfig = serde_yml::from_str("theme: dracula\n").unwrap();
        // Re-picking what the name reads as keeps it.
        config.set_theme_choice(ThemeChoice::Dark);
        assert_eq!(config.theme, "dracula");

        for (choice, stored) in [
            (ThemeChoice::Light, "light"),
            (ThemeChoice::System, "system"),
            (ThemeChoice::Dark, "dark"),
        ] {
            config.set_theme_choice(choice);
            assert_eq!(config.theme, stored);
            let yaml = serde_yml::to_string(&config).unwrap();
            let back: AppConfig = serde_yml::from_str(&yaml).unwrap();
            assert_eq!(back.theme_choice(), choice);
            assert_eq!(back, config);
        }
    }

    #[test]
    fn test_full_config_round_trips_without_active_cave() {
        let mut config = AppConfig {
            theme: "light".into(),
            daily_note_folder: "Journal".into(),
            daily_note_template_slug: Some("day".into()),
            markdown_font: FontConfig {
                font_family: "Iosevka".into(),
                font_size: 15,
            },
            ..AppConfig::default()
        };
        config.agent.max_turns = 3;
        config.agent.rag.embedding_model = Some("BGESmallENV15".into());
        config.agent.tool_config.web_search.api_key = Some("BSA-key".into());
        config.agent.disabled_tools = vec!["web_fetch".into()];

        let yaml = serde_yml::to_string(&config).unwrap();
        let back: AppConfig = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(back, config);
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
