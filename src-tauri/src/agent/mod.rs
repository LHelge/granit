mod error;
pub(crate) mod tools;
pub(crate) mod vectordb;

pub(crate) use crate::commands::SharedCave;
pub use error::AgentError;
use rig_core::message::ToolCall;

use granit_types::AttachedNote;

use std::collections::{HashSet, VecDeque};

use granit_types::{AgentConfig, AgentMode, ProviderConfig, ToolCallInfo};
use rig_core::client::{CompletionClient, Nothing};
use rig_core::completion::message::Message;
use rig_core::providers::{anthropic, mistral, ollama, openai};
use rig_core::tool::ToolDyn;

use self::vectordb::CaveVectorIndex;

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
use granit_types::default_system_prompt;

/// Provider-agnostic agent wrapping different rig-core agent types.
pub(crate) enum ProviderAgent {
    Ollama(rig_core::agent::Agent<ollama::CompletionModel>),
    Anthropic(rig_core::agent::Agent<anthropic::completion::CompletionModel>),
    Mistral(rig_core::agent::Agent<mistral::CompletionModel>),
    OpenAiCompatible(rig_core::agent::Agent<openai::completion::CompletionModel>),
}

/// Dispatch a single expression over all `ProviderAgent` variants.
/// The body is identical for every arm; `$agent` is bound to the inner value.
macro_rules! provider_dispatch {
    ($self:expr, $agent:ident => $body:expr) => {
        match $self {
            ProviderAgent::Ollama($agent) => $body,
            ProviderAgent::Anthropic($agent) => $body,
            ProviderAgent::Mistral($agent) => $body,
            ProviderAgent::OpenAiCompatible($agent) => $body,
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
            ProviderAgent::OpenAiCompatible($agent) => ProviderAgent::OpenAiCompatible($expr),
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
            Self::OpenAiCompatible(_) => "OpenAiCompatible",
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
    max_turns: usize,
}

impl Agent {
    /// Build a provider-agnostic agent from the provided config and shared cave handle.
    pub fn from_config(
        config: &AgentConfig,
        cave: SharedCave,
        vector_index: Option<CaveVectorIndex>,
    ) -> Result<Self, AgentError> {
        if config.providers.is_empty() {
            return Err(AgentError::NoProviders);
        }
        let entry = config.providers.get(config.selected_provider).ok_or(
            AgentError::ProviderIndexOutOfRange(config.selected_provider),
        )?;

        if config.max_history == 0 {
            return Err(AgentError::Build(
                "Max history must be greater than 0".to_string(),
            ));
        }

        if config.max_turns == 0 {
            return Err(AgentError::Build(
                "Max turns must be greater than 0".to_string(),
            ));
        }

        for provider in &config.providers {
            provider.validate().map_err(AgentError::Build)?;
        }

        let cave_tools = tools::build_toolset(cave, config);
        let model = config
            .selected_model
            .as_deref()
            .unwrap_or(entry.provider.default_model());
        let system_prompt = match &config.system_prompt {
            Some(custom) if !custom.trim().is_empty() => custom.clone(),
            _ => default_system_prompt(),
        };
        let system_prompt = format!(
            "{system_prompt}\n\nToday's date is {}.",
            chrono::Local::now().format("%Y-%m-%d")
        );
        let rag_top_n = config.rag.top_n;
        // RAG context is only used in Ask mode.
        let vector_index = match config.mode {
            AgentMode::Ask => vector_index,
            AgentMode::Agent => None,
        };
        // Each arm only constructs the provider-specific client; the shared
        // builder chain lives in `configure_agent`.
        let inner = match &entry.provider {
            ProviderConfig::Ollama { base_url } => {
                let base_url = base_url.as_deref().unwrap_or(DEFAULT_OLLAMA_BASE_URL);
                let client = ollama::Client::builder()
                    .api_key(Nothing)
                    .base_url(base_url)
                    .build()?;
                ProviderAgent::Ollama(configure_agent(
                    client.agent(model),
                    cave_tools,
                    &system_prompt,
                    vector_index,
                    rag_top_n,
                ))
            }
            ProviderConfig::Anthropic { api_key } => {
                let client = anthropic::Client::builder().api_key(api_key).build()?;
                ProviderAgent::Anthropic(configure_agent(
                    client.agent(model),
                    cave_tools,
                    &system_prompt,
                    vector_index,
                    rag_top_n,
                ))
            }
            ProviderConfig::Mistral { api_key } => {
                let client = mistral::Client::builder().api_key(api_key).build()?;
                ProviderAgent::Mistral(configure_agent(
                    client.agent(model),
                    cave_tools,
                    &system_prompt,
                    vector_index,
                    rag_top_n,
                ))
            }
            ProviderConfig::OpenAiCompatible { api_key, base_url } => {
                let client = openai::CompletionsClient::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()?;
                ProviderAgent::OpenAiCompatible(configure_agent(
                    client.agent(model),
                    cave_tools,
                    &system_prompt,
                    vector_index,
                    rag_top_n,
                ))
            }
        };

        Ok(Self {
            inner,
            history: VecDeque::new(),
            max_history: config.max_history,
            max_turns: config.max_turns,
        })
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
    ) -> Result<AgentStream, AgentError> {
        use futures::StreamExt;
        use rig_core::streaming::StreamingPrompt;

        fn map_item<R>(item: rig_core::agent::MultiTurnStreamItem<R>) -> AgentStreamItem {
            use rig_core::agent::MultiTurnStreamItem;
            use rig_core::streaming::{StreamedAssistantContent, StreamedUserContent};

            match item {
                MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t)) => {
                    AgentStreamItem::Text(t.text)
                }
                MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall {
                    tool_call,
                    ..
                }) => AgentStreamItem::ToolCall(build_tool_call_info(tool_call)),
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
                .multi_turn(self.max_turns)
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

/// Apply the provider-independent agent configuration — preamble, toolset,
/// and optional RAG dynamic context — to a rig agent builder.
fn configure_agent<M: rig_core::completion::CompletionModel>(
    builder: rig_core::agent::AgentBuilder<M>,
    cave_tools: Vec<Box<dyn ToolDyn>>,
    system_prompt: &str,
    vector_index: Option<CaveVectorIndex>,
    rag_top_n: usize,
) -> rig_core::agent::Agent<M> {
    let mut builder = builder.preamble(system_prompt).tools(cave_tools);
    if let Some(index) = vector_index {
        builder = builder.dynamic_context(rag_top_n, index);
    }
    builder.build()
}

/// Query the selected provider's API for available models.
pub(crate) async fn list_models(
    provider: &ProviderConfig,
) -> Result<Vec<granit_types::ModelInfo>, AgentError> {
    use rig_core::client::ModelListingClient;

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
        ProviderConfig::OpenAiCompatible { api_key, base_url } => {
            let client = openai::Client::builder()
                .api_key(api_key)
                .base_url(base_url)
                .build()?;
            client.list_models().await?
        }
    };

    let models = dedup_model_infos(models.into_iter().map(|m| granit_types::ModelInfo {
        id: m.id,
        name: m.name,
    }));

    Ok(models)
}

fn dedup_model_infos(
    models: impl IntoIterator<Item = granit_types::ModelInfo>,
) -> Vec<granit_types::ModelInfo> {
    let mut seen_ids = HashSet::new();

    models
        .into_iter()
        // Keep the first occurrence of each id across the full unsorted list.
        .filter(|model| seen_ids.insert(model.id.clone()))
        .collect()
}

fn build_tool_call_info(call: ToolCall) -> ToolCallInfo {
    let name = call.function.name.clone();
    let args = &call.function.arguments;

    let key = match name.as_str() {
        "read_note" | "update_note" | "delete_note" | "edit_note" | "move_note" | "rename_note" => {
            Some("slug")
        }
        "search_notes" | "search_content" | "web_search" => Some("query"),
        "web_fetch" => Some("url"),
        "create_note" => Some("name"),
        "create_folder" | "rename_folder" | "move_folder" | "delete_folder" => Some("path"),
        _ => None,
    };

    let param = key.and_then(|k| args.get(k)?.as_str().map(String::from));

    ToolCallInfo { name, param }
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
    /// Maximum time to wait between successive stream items before giving up.
    /// Chosen to be generous enough for slow tool calls and long inference
    /// turns while still bounding a stuck connection.
    const ITEM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

    /// Consume the stream, collecting the full response text.
    /// Calls `on_chunk` for each text chunk received, and `on_tool` for
    /// tool call / tool result items.
    ///
    /// Each wait for the next item is bounded by [`Self::ITEM_TIMEOUT`]; if
    /// the provider produces nothing in that window the stream is aborted
    /// with [`AgentError::StreamTimeout`].
    pub async fn collect_with(
        &mut self,
        mut on_chunk: impl FnMut(&str),
        mut on_tool: impl FnMut(AgentStreamItem),
    ) -> Result<String, AgentError> {
        use futures::StreamExt;

        let mut full_response = String::new();
        loop {
            let next = match tokio::time::timeout(Self::ITEM_TIMEOUT, self.next()).await {
                Ok(item) => item,
                Err(_) => {
                    return Err(AgentError::StreamTimeout(Self::ITEM_TIMEOUT.as_secs()));
                }
            };
            match next {
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

pub(crate) fn build_agent_prompt(msg: &str, attached_notes: &[AttachedNote]) -> String {
    if attached_notes.is_empty() {
        return msg.to_string();
    }

    let attachment_size: usize = attached_notes
        .iter()
        .map(|note| {
            note.slug.len() + note.content.len() + note.selected.as_deref().map_or(0, str::len)
        })
        .sum();
    let mut prompt = String::with_capacity(msg.len() + attachment_size + 256);
    prompt.push_str(
        "Use the attached note contexts below for this turn. If the user refers to an attached note or selected text, use these attachments directly.\n\n",
    );
    prompt.push_str("<attached_notes>\n");

    for attached_note in attached_notes {
        prompt.push_str("<attached_note>\n");
        prompt.push_str("<slug>");
        prompt.push_str(&attached_note.slug);
        prompt.push_str("</slug>\n");

        if let Some(selected) = attached_note.selected.as_deref() {
            prompt.push_str("<selected_text>\n");
            prompt.push_str(selected);
            prompt.push_str("\n</selected_text>\n");
        }

        prompt.push_str("<content>\n");
        prompt.push_str(&attached_note.content);
        prompt.push_str("\n</content>\n");
        prompt.push_str("</attached_note>\n");
    }

    prompt.push_str("</attached_notes>\n\n");
    prompt.push_str("<user_request>\n");
    prompt.push_str(msg);
    prompt.push_str("\n</user_request>");

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::{
        AgentMode, AttachedNote, ModelInfo, ProviderConfig, ProviderEntry, RagConfig, ToolsConfig,
    };

    fn empty_cave() -> SharedCave {
        std::sync::Arc::new(parking_lot::Mutex::new(None))
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
            mode: AgentMode::default(),
            max_history: 100,
            max_turns: 10,
            system_prompt: None,
            disabled_tools: Vec::new(),
            tool_config: ToolsConfig::default(),
            rag: RagConfig::default(),
        }
    }

    fn provider_config(entry: ProviderEntry) -> AgentConfig {
        AgentConfig {
            providers: vec![entry],
            selected_provider: 0,
            selected_model: None,
            mode: AgentMode::default(),
            max_history: 100,
            max_turns: 10,
            system_prompt: None,
            disabled_tools: Vec::new(),
            tool_config: ToolsConfig::default(),
            rag: RagConfig::default(),
        }
    }

    #[tokio::test]
    async fn agent_builds_from_default_config() {
        let config = AgentConfig::default();
        let agent = Agent::from_config(&config, empty_cave(), None);
        assert!(agent.is_ok(), "Agent should build without error");
    }

    #[tokio::test]
    async fn agent_builds_with_custom_base_url() {
        let config = ollama_config(Some("http://192.168.1.10:11434"));
        let agent = Agent::from_config(&config, empty_cave(), None);
        assert!(agent.is_ok(), "Agent should build with a custom base URL");
    }

    #[tokio::test]
    async fn no_providers_returns_error() {
        let config = AgentConfig {
            providers: vec![],
            selected_provider: 0,
            selected_model: None,
            mode: AgentMode::default(),
            max_history: 100,
            max_turns: 10,
            system_prompt: None,
            disabled_tools: Vec::new(),
            tool_config: ToolsConfig::default(),
            rag: RagConfig::default(),
        };
        let result = Agent::from_config(&config, empty_cave(), None);
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
        let result = Agent::from_config(&config, empty_cave(), None);
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
        let result = Agent::from_config(&config, empty_cave(), None);
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
        let result = Agent::from_config(&config, empty_cave(), None);
        assert!(result.is_ok(), "Agent should build with Mistral API key");
    }

    #[tokio::test]
    async fn openai_compatible_builds_with_api_key_and_base_url() {
        let config = provider_config(ProviderEntry {
            name: None,
            provider: ProviderConfig::OpenAiCompatible {
                api_key: "test-key-789".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
            },
        });
        let result = Agent::from_config(&config, empty_cave(), None);
        assert!(
            result.is_ok(),
            "Agent should build with OpenAI-compatible config"
        );
    }

    #[test]
    fn dedup_model_infos_keeps_first_model_per_id() {
        let models = vec![
            ModelInfo {
                id: "alpha".to_string(),
                name: Some("Alpha".to_string()),
            },
            ModelInfo {
                id: "beta".to_string(),
                name: Some("Beta".to_string()),
            },
            ModelInfo {
                id: "alpha".to_string(),
                name: Some("Alpha Duplicate".to_string()),
            },
        ];

        let deduped = dedup_model_infos(models);

        assert_eq!(
            deduped,
            vec![
                ModelInfo {
                    id: "alpha".to_string(),
                    name: Some("Alpha".to_string()),
                },
                ModelInfo {
                    id: "beta".to_string(),
                    name: Some("Beta".to_string()),
                },
            ]
        );
    }

    #[test]
    fn test_build_agent_prompt_without_attachment() {
        let prompt = build_agent_prompt("Summarize this", &[]);

        assert_eq!(prompt, "Summarize this");
    }

    #[test]
    fn test_build_agent_prompt_with_attachments_and_selection() {
        let prompt = build_agent_prompt(
            "Summarize this",
            &[
                AttachedNote {
                    slug: "daily-note".into(),
                    content: "# Heading\n\nBody".into(),
                    selected: Some("Heading".into()),
                },
                AttachedNote {
                    slug: "shopping".into(),
                    content: "Milk\nEggs".into(),
                    selected: None,
                },
            ],
        );

        assert!(prompt.contains("Use the attached note contexts below for this turn"));
        assert!(prompt.contains("<attached_notes>"));
        assert!(prompt.contains("<slug>daily-note</slug>"));
        assert!(prompt.contains("<slug>shopping</slug>"));
        assert!(prompt.contains("<selected_text>\nHeading\n</selected_text>"));
        assert!(prompt.contains("<content>\n# Heading\n\nBody\n</content>"));
        assert!(prompt.contains("<content>\nMilk\nEggs\n</content>"));
        assert!(prompt.contains("<user_request>\nSummarize this\n</user_request>"));
    }
}
