mod error;

pub use error::AgentError;

use std::collections::VecDeque;

use crate::config::Secrets;
use granit_types::AgentConfig;
use rig::client::{CompletionClient, Nothing};
use rig::completion::message::Message;
use rig::providers::{anthropic, mistral, ollama};

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_SYSTEM_PROMPT: &str =
    "You are a helpful assistant integrated into Granit, a personal note-taking app.";

/// Provider-agnostic agent wrapping different rig-core agent types.
pub(crate) enum ProviderAgent {
    Ollama(rig::agent::Agent<ollama::CompletionModel>),
    Anthropic(rig::agent::Agent<anthropic::completion::CompletionModel>),
    Mistral(rig::agent::Agent<mistral::CompletionModel>),
}

impl Clone for ProviderAgent {
    fn clone(&self) -> Self {
        match self {
            Self::Ollama(a) => Self::Ollama(a.clone()),
            Self::Anthropic(a) => Self::Anthropic(a.clone()),
            Self::Mistral(a) => Self::Mistral(a.clone()),
        }
    }
}

impl std::fmt::Debug for ProviderAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama(_) => write!(f, "ProviderAgent::Ollama"),
            Self::Anthropic(_) => write!(f, "ProviderAgent::Anthropic"),
            Self::Mistral(_) => write!(f, "ProviderAgent::Mistral"),
        }
    }
}

/// An agent backed by a configurable LLM provider, with session conversation history.
#[derive(Clone, Debug)]
pub struct Agent {
    inner: ProviderAgent,
    pub history: VecDeque<Message>,
    max_history: usize,
}

impl Agent {
    /// Build a provider-agnostic agent from the provided config and secrets.
    pub fn from_config(config: &AgentConfig, secrets: &Secrets) -> Result<Self, AgentError> {
        let inner = match config.provider.as_str() {
            "ollama" => Self::build_ollama(config)?,
            "anthropic" => Self::build_anthropic(config, secrets)?,
            "mistral" => Self::build_mistral(config, secrets)?,
            other => return Err(AgentError::UnknownProvider(other.to_string())),
        };

        Ok(Self {
            inner,
            history: VecDeque::new(),
            max_history: config.max_history,
        })
    }

    fn build_ollama(config: &AgentConfig) -> Result<ProviderAgent, AgentError> {
        let base_url = config
            .base_url
            .as_deref()
            .unwrap_or(DEFAULT_OLLAMA_BASE_URL);

        let client = ollama::Client::builder()
            .api_key(Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| AgentError::Build(e.to_string()))?;

        let agent = client
            .agent(&config.model)
            .preamble(DEFAULT_SYSTEM_PROMPT)
            .build();

        Ok(ProviderAgent::Ollama(agent))
    }

    fn build_anthropic(
        config: &AgentConfig,
        secrets: &Secrets,
    ) -> Result<ProviderAgent, AgentError> {
        let api_key = secrets
            .get("ANTHROPIC_API_KEY")
            .ok_or(AgentError::MissingApiKey(
                "Anthropic API key not configured — set ANTHROPIC_API_KEY in secrets".to_string(),
            ))?;

        let client = anthropic::Client::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| AgentError::Build(e.to_string()))?;

        let agent = client
            .agent(&config.model)
            .preamble(DEFAULT_SYSTEM_PROMPT)
            .build();

        Ok(ProviderAgent::Anthropic(agent))
    }

    fn build_mistral(
        config: &AgentConfig,
        secrets: &Secrets,
    ) -> Result<ProviderAgent, AgentError> {
        let api_key = secrets
            .get("MISTRAL_API_KEY")
            .ok_or(AgentError::MissingApiKey(
                "Mistral API key not configured \u{2014} set MISTRAL_API_KEY in secrets".to_string(),
            ))?;

        let client = mistral::Client::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| AgentError::Build(e.to_string()))?;

        let agent = client
            .agent(&config.model)
            .preamble(DEFAULT_SYSTEM_PROMPT)
            .build();

        Ok(ProviderAgent::Mistral(agent))
    }

    /// Push a message to the conversation history, dropping the oldest
    /// messages if the limit is exceeded.
    pub fn push_history(&mut self, message: Message) {
        self.history.push_back(message);
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Clone the agent and snapshot the current history for use across await points.
    pub fn snapshot(&self) -> (Self, Vec<Message>) {
        (self.clone(), self.history.iter().cloned().collect())
    }

    /// Stream a prompt through the underlying provider agent.
    ///
    /// Returns a type-erased stream that yields the same items
    /// regardless of provider.
    pub async fn stream_with_history(
        &self,
        prompt: &str,
        history: Vec<Message>,
        max_turns: usize,
    ) -> Result<AgentStream, AgentError> {
        use futures::StreamExt;
        use rig::streaming::StreamingPrompt;

        fn map_item<R>(item: rig::agent::MultiTurnStreamItem<R>) -> AgentStreamItem {
            use rig::agent::MultiTurnStreamItem;
            use rig::streaming::StreamedAssistantContent;

            match item {
                MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t)) => {
                    AgentStreamItem::Text(t.text)
                }
                MultiTurnStreamItem::FinalResponse(_) => AgentStreamItem::Done,
                _ => AgentStreamItem::Other,
            }
        }

        match &self.inner {
            ProviderAgent::Ollama(agent) => {
                let stream = agent
                    .stream_prompt(prompt)
                    .with_history(history)
                    .multi_turn(max_turns)
                    .await;
                Ok(AgentStream {
                    inner: Box::pin(stream.map(|item| {
                        item.map(map_item)
                            .map_err(|e| AgentError::Stream(e.to_string()))
                    })),
                })
            }
            ProviderAgent::Anthropic(agent) => {
                let stream = agent
                    .stream_prompt(prompt)
                    .with_history(history)
                    .multi_turn(max_turns)
                    .await;
                Ok(AgentStream {
                    inner: Box::pin(stream.map(|item| {
                        item.map(map_item)
                            .map_err(|e| AgentError::Stream(e.to_string()))
                    })),
                })
            }
            ProviderAgent::Mistral(agent) => {
                let stream = agent
                    .stream_prompt(prompt)
                    .with_history(history)
                    .multi_turn(max_turns)
                    .await;
                Ok(AgentStream {
                    inner: Box::pin(stream.map(|item| {
                        item.map(map_item)
                            .map_err(|e| AgentError::Stream(e.to_string()))
                    })),
                })
            }
        }
    }
}

/// A type-erased stream item from the agent, independent of provider.
pub enum AgentStreamItem {
    Text(String),
    Done,
    Other,
}

/// A type-erased stream over agent responses.
pub struct AgentStream {
    inner:
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<AgentStreamItem, AgentError>> + Send>>,
}

impl AgentStream {
    /// Consume the stream, collecting the full response text.
    /// Calls `on_chunk` for each text chunk received.
    pub async fn collect_with(
        &mut self,
        mut on_chunk: impl FnMut(&str),
    ) -> Result<String, AgentError> {
        use futures::StreamExt;

        let mut full_response = String::new();
        loop {
            match self.next().await {
                Some(Ok(AgentStreamItem::Text(text))) => {
                    full_response.push_str(&text);
                    on_chunk(&text);
                }
                Some(Ok(AgentStreamItem::Done)) | None => break,
                Some(Err(e)) => return Err(e),
                Some(Ok(AgentStreamItem::Other)) => {}
            }
        }
        Ok(full_response)
    }
}

impl futures::Stream for AgentStream {
    type Item = Result<AgentStreamItem, AgentError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_secrets() -> Secrets {
        Secrets::new(std::collections::HashMap::new())
    }

    #[tokio::test]
    async fn agent_builds_from_default_config() {
        let config = AgentConfig::default();
        let agent = Agent::from_config(&config, &empty_secrets());
        assert!(agent.is_ok(), "Agent should build without error");
    }

    #[tokio::test]
    async fn agent_builds_with_custom_base_url() {
        let config = AgentConfig {
            provider: "ollama".to_string(),
            model: "qwen3.5:9b".to_string(),
            base_url: Some("http://192.168.1.10:11434".to_string()),
            ..AgentConfig::default()
        };
        let agent = Agent::from_config(&config, &empty_secrets());
        assert!(agent.is_ok(), "Agent should build with a custom base URL");
    }

    #[tokio::test]
    async fn agent_unknown_provider_returns_error() {
        let config = AgentConfig {
            provider: "nope".to_string(),
            model: "test".to_string(),
            base_url: None,
            ..AgentConfig::default()
        };
        let result = Agent::from_config(&config, &empty_secrets());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AgentError::UnknownProvider(_)),
            "Expected UnknownProvider, got: {err}"
        );
    }

    #[tokio::test]
    async fn anthropic_missing_api_key_returns_error() {
        let config = AgentConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: None,
            ..AgentConfig::default()
        };
        let result = Agent::from_config(&config, &empty_secrets());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AgentError::MissingApiKey(_)),
            "Expected MissingApiKey, got: {err}"
        );
    }

    #[tokio::test]
    async fn anthropic_builds_with_api_key() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("ANTHROPIC_API_KEY".to_string(), "test-key-123".to_string());
        let secrets = Secrets::new(vars);
        let config = AgentConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: None,
            ..AgentConfig::default()
        };
        let result = Agent::from_config(&config, &secrets);
        assert!(result.is_ok(), "Agent should build with Anthropic API key");
    }

    #[tokio::test]
    async fn mistral_missing_api_key_returns_error() {
        let config = AgentConfig {
            provider: "mistral".to_string(),
            model: "mistral-small-latest".to_string(),
            base_url: None,
            ..AgentConfig::default()
        };
        let result = Agent::from_config(&config, &empty_secrets());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AgentError::MissingApiKey(_)),
            "Expected MissingApiKey, got: {err}"
        );
    }

    #[tokio::test]
    async fn mistral_builds_with_api_key() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("MISTRAL_API_KEY".to_string(), "test-key-456".to_string());
        let secrets = Secrets::new(vars);
        let config = AgentConfig {
            provider: "mistral".to_string(),
            model: "mistral-small-latest".to_string(),
            base_url: None,
            ..AgentConfig::default()
        };
        let result = Agent::from_config(&config, &secrets);
        assert!(result.is_ok(), "Agent should build with Mistral API key");
    }
}
