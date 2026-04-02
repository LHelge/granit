use serde::{Deserialize, Serialize};

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
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
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
}

fn default_max_history() -> usize {
    100
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            providers: vec![ProviderEntry {
                name: None,
                provider: ProviderConfig::Ollama { base_url: None },
            }],
            selected_provider: 0,
            selected_model: None,
            max_history: default_max_history(),
        }
    }
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

/// Lightweight representation of a tool invocation for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallInfo {
    pub name: String,
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
        };

        let yaml = serde_yml::to_string(&config).unwrap();
        let parsed: AgentConfig = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.providers.len(), 2);
        assert_eq!(parsed.selected_provider, 1);
        assert_eq!(parsed.max_history, 50);
        assert_eq!(parsed.providers[1].name, Some("Claude".into()));
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
}
