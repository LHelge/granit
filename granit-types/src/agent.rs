use serde::{Deserialize, Serialize};

use crate::NOTE_ICONS;

const SYSTEM_PROMPT_BASE: &str = r#"<|think|> You are a helpful assistant integrated into Granit, a personal note-taking app. 
The notes are stored in markdown format in a 'cave' on the user's local filesystem and are identified by a unique slug (filename without .md extension).
You can link the user to existing notes by using wiki-style links like [[slug]]. 
You can call tools work with the notes. Always try to use the tools for any note operations instead of asking the user to do it manually. 
Be mindful that edits should only replace text in the body of the note, not the frontmatter.
Daily notes use the YYYY-MM-DD date format as their slug (e.g. 2026-04-05) and are stored in a configurable folder (default: "Daily"). Use the open_daily_note tool to create or open them."#;

/// Build the default system prompt including available note icon IDs.
pub fn default_system_prompt() -> String {
    let ids: Vec<&str> = NOTE_ICONS.iter().map(|e| e.id).collect();
    format!(
        "{}\n\nWhen creating or updating notes you can optionally set an icon using one of these IDs:\n{}",
        SYSTEM_PROMPT_BASE,
        ids.join(", ")
    )
}

/// Tagged provider configuration.
///
/// Each variant carries only the fields relevant to that provider.
/// Uses `#[serde(tag = "provider")]` for clean YAML like:
/// ```yaml
/// providers:
///   - provider: ollama
///     base_url: http://localhost:11434
///   - provider: anthropic
///     api_key: sk-ant-...
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ProviderConfig {
    Ollama {
        #[serde(skip_serializing_if = "Option::is_none")]
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
    },
    Mistral {
        api_key: String,
    },
    Prisma {
        api_key: String,
    },
}

impl ProviderConfig {
    /// Default model ID for this provider.
    pub fn default_model(&self) -> &str {
        match self {
            Self::Ollama { .. } => "qwen3.5:9b",
            Self::Anthropic { .. } => "claude-sonnet-4-6",
            Self::Mistral { .. } => "mistral-small-latest",
            Self::Prisma { .. } => "prisma_default",
        }
    }

    /// Short lowercase label for the provider variant (e.g. `"ollama"`).
    pub fn provider_type(&self) -> &str {
        match self {
            Self::Ollama { .. } => "ollama",
            Self::Anthropic { .. } => "anthropic",
            Self::Mistral { .. } => "mistral",
            Self::Prisma { .. } => "prisma",
        }
    }
}

/// A provider entry with an optional user-defined name for disambiguation.
///
/// Wraps `ProviderConfig` and flattens it so YAML stays clean:
/// ```yaml
/// - name: "My Local Ollama"
///   provider: ollama
///   base_url: http://localhost:11434
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(flatten)]
    pub provider: ProviderConfig,
}

impl ProviderEntry {
    /// User-friendly label for this provider.
    ///
    /// Returns the user-defined `name` if set, otherwise derives one from
    /// the provider variant and its distinguishing fields.
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        match &self.provider {
            ProviderConfig::Ollama { base_url, .. } => {
                format!(
                    "Ollama ({})",
                    base_url.as_deref().unwrap_or("localhost:11434")
                )
            }
            ProviderConfig::Anthropic { .. } => "Anthropic".to_string(),
            ProviderConfig::Mistral { .. } => "Mistral".to_string(),
            ProviderConfig::Prisma { .. } => "Prisma".to_string(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match &self.provider {
            ProviderConfig::Ollama { .. } => Ok(()),
            ProviderConfig::Anthropic { api_key } => validate_api_key("Anthropic", api_key),
            ProviderConfig::Mistral { api_key } => validate_api_key("Mistral", api_key),
            ProviderConfig::Prisma { api_key } => validate_api_key("Prisma", api_key),
        }
    }
}

/// Configuration for the `web_search` tool (Brave Search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "WebSearchConfig::default_max_results")]
    pub max_results: usize,
}

impl WebSearchConfig {
    fn default_max_results() -> usize {
        5
    }
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            max_results: Self::default_max_results(),
        }
    }
}

/// Configuration for the `web_fetch` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchConfig {
    #[serde(default = "WebFetchConfig::default_max_output_chars")]
    pub max_output_chars: usize,
}

impl WebFetchConfig {
    fn default_max_output_chars() -> usize {
        100_000
    }
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            max_output_chars: Self::default_max_output_chars(),
        }
    }
}

/// Per-tool configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default)]
    pub web_search: WebSearchConfig,
    #[serde(default)]
    pub web_fetch: WebFetchConfig,
}

/// RAG (Retrieval-Augmented Generation) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Whether RAG is enabled.
    #[serde(default = "RagConfig::default_enabled")]
    pub enabled: bool,
    /// Number of similar notes to retrieve per agent query.
    #[serde(default = "RagConfig::default_top_n")]
    pub top_n: usize,
    /// fastembed model identifier (e.g. "AllMiniLML6V2").
    /// When `None`, the built-in default model is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

impl RagConfig {
    fn default_enabled() -> bool {
        true
    }

    fn default_top_n() -> usize {
        5
    }
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: Self::default_enabled(),
            top_n: Self::default_top_n(),
            embedding_model: None,
        }
    }
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "AgentConfig::default_providers")]
    pub providers: Vec<ProviderEntry>,
    #[serde(default)]
    pub selected_provider: usize,
    /// Last-used model ID. If `None`, the agent uses a provider-specific default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    /// Maximum number of messages to retain in chat history.
    /// Oldest messages are dropped when the limit is exceeded.
    #[serde(default = "default_max_history")]
    pub max_history: usize,
    /// Maximum multi-turn tool-call rounds per prompt.
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,
    /// Optional user-defined system prompt override.
    /// When `None`, the built-in default is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Tool names that should NOT be registered with the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,
    /// Per-tool configuration (API keys, limits, etc.).
    #[serde(default)]
    pub tool_config: ToolsConfig,
    /// RAG configuration.
    #[serde(default)]
    pub rag: RagConfig,
}

fn default_max_history() -> usize {
    100
}

fn default_max_turns() -> usize {
    10
}

impl AgentConfig {
    fn default_providers() -> Vec<ProviderEntry> {
        vec![ProviderEntry {
            name: None,
            provider: ProviderConfig::Ollama { base_url: None },
        }]
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.providers.is_empty() {
            return Err("At least one provider must be configured".to_string());
        }

        if self.selected_provider >= self.providers.len() {
            return Err(format!(
                "Selected provider index {} is out of range for {} configured provider(s)",
                self.selected_provider,
                self.providers.len()
            ));
        }

        for (index, entry) in self.providers.iter().enumerate() {
            entry
                .validate()
                .map_err(|err| format!("Provider {}: {err}", index + 1))?;
        }

        if self.max_history == 0 {
            return Err("Max history must be greater than 0".to_string());
        }

        if self.max_turns == 0 {
            return Err("Max turns must be greater than 0".to_string());
        }

        Ok(())
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            providers: Self::default_providers(),
            selected_provider: 0,
            selected_model: None,
            max_history: default_max_history(),
            max_turns: default_max_turns(),
            system_prompt: None,
            disabled_tools: Vec::new(),
            tool_config: ToolsConfig::default(),
            rag: RagConfig::default(),
        }
    }
}

fn validate_api_key(provider: &str, api_key: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err(format!("{provider} provider requires an API key"));
    }

    Ok(())
}

/// Lightweight model information returned by a provider's API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ModelInfo {
    /// Returns `name` if set, otherwise falls back to `id`.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

/// Summary of a configured provider, for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderInfo {
    pub index: usize,
    pub display_name: String,
    pub provider_type: String,
}

/// A single message in the agent chat history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachedNote {
    pub slug: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
}

/// Lightweight representation of a tool invocation for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallInfo {
    pub name: String,
    pub param: Option<String>,
}

/// Metadata about an available agent tool, for the settings UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_config_yaml_round_trip() {
        let entry = ProviderEntry {
            name: Some("My Ollama".into()),
            provider: ProviderConfig::Ollama {
                base_url: Some("http://localhost:11434".into()),
            },
        };
        let yaml = serde_yml::to_string(&entry).unwrap();
        assert!(yaml.contains("provider: ollama"));
        assert!(yaml.contains("base_url:"));
        assert!(yaml.contains("name: My Ollama"));

        let parsed: ProviderEntry = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn provider_config_all_variants_round_trip() {
        let entries = vec![
            ProviderEntry {
                name: None,
                provider: ProviderConfig::Ollama { base_url: None },
            },
            ProviderEntry {
                name: None,
                provider: ProviderConfig::Anthropic {
                    api_key: "sk-test".into(),
                },
            },
            ProviderEntry {
                name: None,
                provider: ProviderConfig::Mistral {
                    api_key: "mist-test".into(),
                },
            },
            ProviderEntry {
                name: None,
                provider: ProviderConfig::Prisma {
                    api_key: "prisma-test".into(),
                },
            },
        ];

        let yaml = serde_yml::to_string(&entries).unwrap();
        let parsed: Vec<ProviderEntry> = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed, entries);
    }

    #[test]
    fn provider_entry_display_name_uses_custom_name() {
        let entry = ProviderEntry {
            name: Some("Work Anthropic".into()),
            provider: ProviderConfig::Anthropic {
                api_key: "key".into(),
            },
        };
        assert_eq!(entry.display_name(), "Work Anthropic");
    }

    #[test]
    fn provider_entry_display_name_derived() {
        let ollama = ProviderEntry {
            name: None,
            provider: ProviderConfig::Ollama {
                base_url: Some("http://myhost:1234".into()),
            },
        };
        assert_eq!(ollama.display_name(), "Ollama (http://myhost:1234)");

        let ollama_default = ProviderEntry {
            name: None,
            provider: ProviderConfig::Ollama { base_url: None },
        };
        assert_eq!(ollama_default.display_name(), "Ollama (localhost:11434)");

        let anthropic = ProviderEntry {
            name: None,
            provider: ProviderConfig::Anthropic {
                api_key: "key".into(),
            },
        };
        assert_eq!(anthropic.display_name(), "Anthropic");

        let mistral = ProviderEntry {
            name: None,
            provider: ProviderConfig::Mistral {
                api_key: "key".into(),
            },
        };
        assert_eq!(mistral.display_name(), "Mistral");

        let prisma = ProviderEntry {
            name: None,
            provider: ProviderConfig::Prisma {
                api_key: "key".into(),
            },
        };
        assert_eq!(prisma.display_name(), "Prisma");
    }

    #[test]
    fn agent_config_default_has_one_ollama() {
        let config = AgentConfig::default();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.selected_provider, 0);
        assert_eq!(config.max_history, 100);
        assert!(matches!(
            config.providers[0].provider,
            ProviderConfig::Ollama { .. }
        ));
    }

    #[test]
    fn agent_config_yaml_round_trip() {
        let config = AgentConfig {
            providers: vec![
                ProviderEntry {
                    name: None,
                    provider: ProviderConfig::Ollama { base_url: None },
                },
                ProviderEntry {
                    name: Some("Claude".into()),
                    provider: ProviderConfig::Anthropic {
                        api_key: "sk-ant-test".into(),
                    },
                },
            ],
            selected_provider: 1,
            selected_model: Some("claude-sonnet-4-20250514".into()),
            max_history: 50,
            max_turns: 5,
            system_prompt: Some("You are helpful.".into()),
            disabled_tools: vec!["delete_note".into()],
            tool_config: ToolsConfig {
                web_search: WebSearchConfig {
                    api_key: Some("BSA-test-key".into()),
                    max_results: 10,
                },
                web_fetch: WebFetchConfig {
                    max_output_chars: 50_000,
                },
            },
            rag: RagConfig::default(),
        };

        let yaml = serde_yml::to_string(&config).unwrap();
        let parsed: AgentConfig = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.providers.len(), 2);
        assert_eq!(parsed.selected_provider, 1);
        assert_eq!(parsed.max_history, 50);
        assert_eq!(parsed.providers[1].name, Some("Claude".into()));
    }

    #[test]
    fn provider_validation_requires_api_keys_when_needed() {
        let entry = ProviderEntry {
            name: None,
            provider: ProviderConfig::Anthropic {
                api_key: "   ".into(),
            },
        };

        let err = entry.validate().expect_err("validation should fail");
        assert!(err.contains("requires an API key"));
    }

    #[test]
    fn agent_config_validation_rejects_empty_provider_list() {
        let config = AgentConfig {
            providers: Vec::new(),
            ..AgentConfig::default()
        };

        let err = config.validate().expect_err("validation should fail");
        assert!(err.contains("At least one provider"));
    }

    #[test]
    fn agent_config_validation_rejects_invalid_selected_provider() {
        let config = AgentConfig {
            selected_provider: 5,
            ..AgentConfig::default()
        };

        let err = config.validate().expect_err("validation should fail");
        assert!(err.contains("out of range"));
    }

    #[test]
    fn model_info_display_name() {
        let with_name = ModelInfo {
            id: "gpt-4".into(),
            name: Some("GPT-4 Turbo".into()),
        };
        assert_eq!(with_name.display_name(), "GPT-4 Turbo");

        let without_name = ModelInfo {
            id: "llama3:8b".into(),
            name: None,
        };
        assert_eq!(without_name.display_name(), "llama3:8b");
    }

    #[test]
    fn model_info_json_round_trip() {
        let model = ModelInfo {
            id: "claude-3-opus".into(),
            name: Some("Claude 3 Opus".into()),
        };
        let json = serde_json::to_string(&model).unwrap();
        let parsed: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, model);
    }

    #[test]
    fn attached_note_round_trip_with_selection() {
        let note = AttachedNote {
            slug: "daily-note".into(),
            content: "# Heading\n\nBody".into(),
            selected: Some("Heading".into()),
        };

        let json = serde_json::to_string(&note).unwrap();
        let parsed: AttachedNote = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, note);
    }

    #[test]
    fn attached_note_omits_empty_selection() {
        let note = AttachedNote {
            slug: "daily-note".into(),
            content: "Body".into(),
            selected: None,
        };

        let json = serde_json::to_string(&note).unwrap();

        assert!(json.contains("\"slug\":\"daily-note\""));
        assert!(!json.contains("selected"));
    }
}
