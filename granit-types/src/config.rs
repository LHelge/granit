use crate::AgentConfig;
use serde::{Deserialize, Serialize};

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

/// Application configuration as returned over IPC.
///
/// Paths are represented as strings for cross-platform serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub recent_caves: Vec<String>,
    pub agent: AgentConfig,
    pub markdown_font: FontConfig,
    pub reading_font: FontConfig,
    pub agent_font: FontConfig,
    pub sidebar: SidebarConfig,
    pub agent_panel: SidebarConfig,
    /// The currently open cave path, if any. Runtime-only — not persisted.
    pub active_cave: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            recent_caves: Vec::new(),
            agent: AgentConfig::default(),
            markdown_font: FontConfig::markdown_default(),
            reading_font: FontConfig::reading_default(),
            agent_font: FontConfig::agent_default(),
            sidebar: SidebarConfig::sidebar_default(),
            agent_panel: SidebarConfig::agent_default(),
            active_cave: None,
        }
    }
}
