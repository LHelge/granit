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
