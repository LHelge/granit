mod error;
pub(crate) mod tools;

pub use error::AgentError;
pub use tools::SharedCave;

use std::collections::VecDeque;

use granit_types::{AgentConfig, ProviderConfig, ToolCallInfo};
use rig::client::{CompletionClient, Nothing};
use rig::completion::message::Message;
use rig::providers::{anthropic, mistral, ollama, openai};
use rig::tool::ToolDyn;

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_PRISMA_BASE_URL: &str = "https://api.ai.auth.axis.cloud/v1";
const SYSTEM_PROMPT_BASE: &str = r#"<|think|> You are a helpful assistant integrated into Granit, a personal note-taking app. 
The notes are stored in markdown format in a 'cave' on the user's local filesystem and are identified by a unique slug (filename without .md extension).
You can link the user to existing notes by using wiki-style links like [[slug]]. 
You can call tools work with the notes. Always try to use the tools for any note operations instead of asking the user to do it manually. 
Be mindful that edits should only replace text in the body of the note, not the frontmatter."#;

fn build_system_prompt() -> String {
    let ids: Vec<&str> = granit_types::NOTE_ICONS.iter().map(|e| e.id).collect();
    format!(
        "{}\n\nWhen creating or updating notes you can optionally set an icon using one of these IDs:\n{}",
        SYSTEM_PROMPT_BASE,
        ids.join(", ")
    )
}

/// Provider-agnostic agent wrapping different rig-core agent types.
pub(crate) enum ProviderAgent {
    Ollama(rig::agent::Agent<ollama::CompletionModel>),
    Anthropic(rig::agent::Agent<anthropic::completion::CompletionModel>),
    Mistral(rig::agent::Agent<mistral::CompletionModel>),
    Prisma(rig::agent::Agent<openai::completion::CompletionModel>),
}

/// Dispatch a single expression over all `ProviderAgent` variants.
/// The body is identical for every arm; `$agent` is bound to the inner value.
macro_rules! provider_dispatch {
    ($self:expr, $agent:ident => $body:expr) => {
        match $self {
            ProviderAgent::Ollama($agent) => $body,
            ProviderAgent::Anthropic($agent) => $body,
            ProviderAgent::Mistral($agent) => $body,
            ProviderAgent::Prisma($agent) => $body,
        }
    };
}

/// Map the inner value of a `ProviderAgent`, preserving the variant.
macro_rules! provider_map {
    ($self:expr, $agent:ident => $expr:expr) => {
        match $self {
            ProviderAgent::Ollama($agent) => ProviderAgent::Ollama($expr),
            ProviderAgent::Anthropic($agent) => ProviderAgent::Anthropic($expr),
            ProviderAgent::Mistral($agent) => ProviderAgent::Mistral($expr),
            ProviderAgent::Prisma($agent) => ProviderAgent::Prisma($expr),
        }
    };
}

impl Clone for ProviderAgent {
    fn clone(&self) -> Self {
        provider_map!(self, a => a.clone())
    }
}

impl std::fmt::Debug for ProviderAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Ollama(_) => "Ollama",
            Self::Anthropic(_) => "Anthropic",
            Self::Mistral(_) => "Mistral",
            Self::Prisma(_) => "Prisma",
        };
        write!(f, "ProviderAgent::{name}")
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
    /// Build a provider-agnostic agent from the provided config and shared cave handle.
    pub fn from_config(config: &AgentConfig, cave: SharedCave) -> Result<Self, AgentError> {
        if config.providers.is_empty() {
            return Err(AgentError::NoProviders);
        }
        let entry = config.providers.get(config.selected_provider).ok_or(
            AgentError::ProviderIndexOutOfRange(config.selected_provider),
        )?;

        let cave_tools = tools::cave_toolset(cave);
        let model = config
            .selected_model
            .as_deref()
            .unwrap_or(entry.provider.default_model());
        let system_prompt = build_system_prompt();
        let inner = match &entry.provider {
            ProviderConfig::Ollama { base_url } => {
                Self::build_ollama(base_url.as_deref(), model, cave_tools, &system_prompt)?
            }
            ProviderConfig::Anthropic { api_key } => {
                Self::build_anthropic(api_key, model, cave_tools, &system_prompt)?
            }
            ProviderConfig::Mistral { api_key } => {
                Self::build_mistral(api_key, model, cave_tools, &system_prompt)?
            }
            ProviderConfig::Prisma { api_key } => {
                Self::build_prisma(api_key, model, cave_tools, &system_prompt)?
            }
        };

        Ok(Self {
            inner,
            history: VecDeque::new(),
            max_history: config.max_history,
        })
    }

    fn build_ollama(
        base_url: Option<&str>,
        model: &str,
        cave_tools: Vec<Box<dyn ToolDyn>>,
        system_prompt: &str,
    ) -> Result<ProviderAgent, AgentError> {
        let base_url = base_url.unwrap_or(DEFAULT_OLLAMA_BASE_URL);

        let client = ollama::Client::builder()
            .api_key(Nothing)
            .base_url(base_url)
            .build()?;

        let agent = client
            .agent(model)
            .preamble(system_prompt)
            .tools(cave_tools)
            .build();

        Ok(ProviderAgent::Ollama(agent))
    }

    fn build_anthropic(
        api_key: &str,
        model: &str,
        cave_tools: Vec<Box<dyn ToolDyn>>,
        system_prompt: &str,
    ) -> Result<ProviderAgent, AgentError> {
        let client = anthropic::Client::builder().api_key(api_key).build()?;

        let agent = client
            .agent(model)
            .preamble(system_prompt)
            .tools(cave_tools)
            .build();

        Ok(ProviderAgent::Anthropic(agent))
    }

    fn build_prisma(
        api_key: &str,
        model: &str,
        cave_tools: Vec<Box<dyn ToolDyn>>,
        system_prompt: &str,
    ) -> Result<ProviderAgent, AgentError> {
        let client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .base_url(DEFAULT_PRISMA_BASE_URL)
            .build()?;

        let agent = client
            .agent(model)
            .preamble(system_prompt)
            .tools(cave_tools)
            .build();

        Ok(ProviderAgent::Prisma(agent))
    }

    fn build_mistral(
        api_key: &str,
        model: &str,
        cave_tools: Vec<Box<dyn ToolDyn>>,
        system_prompt: &str,
    ) -> Result<ProviderAgent, AgentError> {
        let client = mistral::Client::builder().api_key(api_key).build()?;

        let agent = client
            .agent(model)
            .preamble(system_prompt)
            .tools(cave_tools)
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

    /// Clear all conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
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
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            match item {
                MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t)) => {
                    AgentStreamItem::Text(t.text)
                }
                MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall {
                    tool_call,
                    ..
                }) => AgentStreamItem::ToolCall(ToolCallInfo {
                    name: tool_call.function.name.clone(),
                }),
                MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult { .. }) => {
                    AgentStreamItem::ToolResult
                }
                MultiTurnStreamItem::FinalResponse(_) => AgentStreamItem::Done,
                _ => AgentStreamItem::Other,
            }
        }

        provider_dispatch!(&self.inner, agent => {
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
        })
    }
}

/// Query the selected provider's API for available models.
pub(crate) async fn list_models(
    provider: &ProviderConfig,
) -> Result<Vec<granit_types::ModelInfo>, AgentError> {
    use rig::client::ModelListingClient;

    let models = match provider {
        ProviderConfig::Ollama { base_url } => {
            let base_url = base_url.as_deref().unwrap_or(DEFAULT_OLLAMA_BASE_URL);
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(base_url)
                .build()?;
            client.list_models().await?
        }
        ProviderConfig::Anthropic { api_key } => {
            let client = anthropic::Client::builder().api_key(api_key).build()?;
            client.list_models().await?
        }
        ProviderConfig::Mistral { api_key } => {
            let client = mistral::Client::builder().api_key(api_key).build()?;
            client.list_models().await?
        }
        ProviderConfig::Prisma { api_key } => {
            let client = openai::Client::builder()
                .api_key(api_key)
                .base_url(DEFAULT_PRISMA_BASE_URL)
                .build()?;
            client.list_models().await?
        }
    };

    Ok(models
        .into_iter()
        .map(|m| granit_types::ModelInfo {
            id: m.id,
            name: m.name,
        })
        .collect())
}

/// A type-erased stream item from the agent, independent of provider.
pub enum AgentStreamItem {
    Text(String),
    ToolCall(ToolCallInfo),
    /// A tool has finished executing (used to trigger cave refresh).
    ToolResult,
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
    /// Calls `on_chunk` for each text chunk received, and `on_tool` for
    /// tool call / tool result items.
    pub async fn collect_with(
        &mut self,
        mut on_chunk: impl FnMut(&str),
        mut on_tool: impl FnMut(AgentStreamItem),
    ) -> Result<String, AgentError> {
        use futures::StreamExt;

        let mut full_response = String::new();
        loop {
            match self.next().await {
                Some(Ok(AgentStreamItem::Text(text))) => {
                    full_response.push_str(&text);
                    on_chunk(&text);
                }
                Some(Ok(item @ AgentStreamItem::ToolCall(_)))
                | Some(Ok(item @ AgentStreamItem::ToolResult)) => {
                    on_tool(item);
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
    use granit_types::{ProviderConfig, ProviderEntry};

    fn empty_cave() -> SharedCave {
        std::sync::Arc::new(std::sync::Mutex::new(None))
    }

    fn ollama_config(base_url: Option<&str>) -> AgentConfig {
        AgentConfig {
            providers: vec![ProviderEntry {
                name: None,
                provider: ProviderConfig::Ollama {
                    base_url: base_url.map(String::from),
                },
            }],
            selected_provider: 0,
            selected_model: None,
            max_history: 100,
        }
    }

    fn provider_config(entry: ProviderEntry) -> AgentConfig {
        AgentConfig {
            providers: vec![entry],
            selected_provider: 0,
            selected_model: None,
            max_history: 100,
        }
    }

    #[tokio::test]
    async fn agent_builds_from_default_config() {
        let config = AgentConfig::default();
        let agent = Agent::from_config(&config, empty_cave());
        assert!(agent.is_ok(), "Agent should build without error");
    }

    #[tokio::test]
    async fn agent_builds_with_custom_base_url() {
        let config = ollama_config(Some("http://192.168.1.10:11434"));
        let agent = Agent::from_config(&config, empty_cave());
        assert!(agent.is_ok(), "Agent should build with a custom base URL");
    }

    #[tokio::test]
    async fn no_providers_returns_error() {
        let config = AgentConfig {
            providers: vec![],
            selected_provider: 0,
            selected_model: None,
            max_history: 100,
        };
        let result = Agent::from_config(&config, empty_cave());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AgentError::NoProviders),
            "Expected NoProviders, got: {err}"
        );
    }

    #[tokio::test]
    async fn selected_provider_out_of_range_returns_error() {
        let mut config = ollama_config(None);
        config.selected_provider = 5;
        let result = Agent::from_config(&config, empty_cave());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AgentError::ProviderIndexOutOfRange(5)),
            "Expected ProviderIndexOutOfRange(5), got: {err}"
        );
    }

    #[tokio::test]
    async fn anthropic_builds_with_api_key() {
        let config = provider_config(ProviderEntry {
            name: None,
            provider: ProviderConfig::Anthropic {
                api_key: "test-key-123".to_string(),
            },
        });
        let result = Agent::from_config(&config, empty_cave());
        assert!(result.is_ok(), "Agent should build with Anthropic API key");
    }

    #[tokio::test]
    async fn mistral_builds_with_api_key() {
        let config = provider_config(ProviderEntry {
            name: None,
            provider: ProviderConfig::Mistral {
                api_key: "test-key-456".to_string(),
            },
        });
        let result = Agent::from_config(&config, empty_cave());
        assert!(result.is_ok(), "Agent should build with Mistral API key");
    }

    #[tokio::test]
    async fn prisma_builds_with_api_key() {
        let config = provider_config(ProviderEntry {
            name: None,
            provider: ProviderConfig::Prisma {
                api_key: "test-key-789".to_string(),
            },
        });
        let result = Agent::from_config(&config, empty_cave());
        assert!(result.is_ok(), "Agent should build with Prisma API key");
    }
}
