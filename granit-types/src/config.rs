use serde::{Deserialize, Serialize};

use crate::AgentConfig;

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
            active_cave: None,
        }
    }
}
