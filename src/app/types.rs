use serde::{Deserialize, Serialize};

/// Mirrors the backend `AppConfig` for IPC deserialization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    pub recent_caves: Vec<String>,
    pub agent: AgentConfig,
}

/// Mirrors the backend `AgentConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
        }
    }
}

/// Metadata for a note in the cave.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NoteMeta {
    pub slug: String,
    pub relative_path: String,
}

/// Full note content.
#[derive(Debug, Clone, Deserialize)]
pub struct Note {
    pub meta: NoteMeta,
    pub content: String,
}
