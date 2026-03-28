mod error;

pub use error::AgentError;

use granit_types::AgentConfig;
use rig::client::{CompletionClient, Nothing};
use rig::providers::ollama;

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_SYSTEM_PROMPT: &str =
    "You are a helpful assistant integrated into Granit, a personal note-taking app.";

/// An agent backed by an Ollama model.
pub struct Agent {
    inner: rig::agent::Agent<ollama::CompletionModel>,
}

impl Agent {
    /// Build an Ollama-backed agent from the provided config.
    pub fn from_config(config: &AgentConfig) -> Result<Self, AgentError> {
        let base_url = config
            .base_url
            .as_deref()
            .unwrap_or(DEFAULT_OLLAMA_BASE_URL);

        let client = ollama::Client::builder()
            .api_key(Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| AgentError::Build(e.to_string()))?;

        let inner = client
            .agent(&config.model)
            .preamble(DEFAULT_SYSTEM_PROMPT)
            .build();

        Ok(Self { inner })
    }

    pub fn inner(&self) -> &rig::agent::Agent<ollama::CompletionModel> {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn agent_builds_from_default_config() {
        let config = AgentConfig::default();
        let agent = Agent::from_config(&config);
        assert!(agent.is_ok(), "Agent should build without error");
    }

    #[tokio::test]
    async fn agent_builds_with_custom_base_url() {
        let config = AgentConfig {
            provider: "ollama".to_string(),
            model: "qwen3.5:9b".to_string(),
            base_url: Some("http://192.168.1.10:11434".to_string()),
        };
        let agent = Agent::from_config(&config);
        assert!(agent.is_ok(), "Agent should build with a custom base URL");
    }
}
