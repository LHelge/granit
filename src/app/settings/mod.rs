mod agent;
mod markdown;
mod notes;
mod reading;
mod theme;

use crate::app::components::modal::Modal;
use crate::app::{ipc, AppCtx};
use agent::AgentSettings;
use granit_types::{
    AgentConfig, AppConfig, FontConfig, ProviderConfig, ProviderEntry, RagConfig, ToolInfo,
    ToolsConfig, WebFetchConfig, WebSearchConfig,
};
use leptos::prelude::*;
use markdown::MarkdownSettings;
use notes::NotesSettings;
use reading::ReadingSettings;
use theme::ThemeSettings;

/// Flat representation of one provider for form editing.
#[derive(Clone)]
pub(super) struct ProviderFormEntry {
    pub id: usize,
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl ProviderFormEntry {
    fn from_entry(id: usize, entry: &ProviderEntry) -> Self {
        let (provider_type, base_url, api_key) = match &entry.provider {
            ProviderConfig::Ollama { base_url } => (
                "ollama".into(),
                base_url.clone().unwrap_or_default(),
                String::new(),
            ),
            ProviderConfig::Anthropic { api_key } => {
                ("anthropic".into(), String::new(), api_key.clone())
            }
            ProviderConfig::Mistral { api_key } => {
                ("mistral".into(), String::new(), api_key.clone())
            }
            ProviderConfig::OpenAiCompatible { api_key, base_url } => {
                ("openai".into(), base_url.clone(), api_key.clone())
            }
        };
        Self {
            id,
            provider_type,
            name: entry.name.clone().unwrap_or_default(),
            base_url,
            api_key,
        }
    }

    fn new_default(id: usize, provider_type: &str) -> Self {
        Self {
            id,
            provider_type: provider_type.into(),
            name: String::new(),
            base_url: String::new(),
            api_key: String::new(),
        }
    }

    fn to_provider_entry(&self) -> ProviderEntry {
        let provider = match self.provider_type.as_str() {
            "ollama" => ProviderConfig::Ollama {
                base_url: if self.base_url.trim().is_empty() {
                    None
                } else {
                    Some(self.base_url.clone())
                },
            },
            "anthropic" => ProviderConfig::Anthropic {
                api_key: self.api_key.clone(),
            },
            "mistral" => ProviderConfig::Mistral {
                api_key: self.api_key.clone(),
            },
            "openai" => ProviderConfig::OpenAiCompatible {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
            },
            _ => ProviderConfig::Ollama { base_url: None },
        };
        ProviderEntry {
            name: if self.name.trim().is_empty() {
                None
            } else {
                Some(self.name.clone())
            },
            provider,
        }
    }

    /// Human label for the provider type.
    pub fn type_label(&self) -> &str {
        match self.provider_type.as_str() {
            "ollama" => "Ollama",
            "anthropic" => "Anthropic",
            "mistral" => "Mistral",
            "openai" => "OpenAI",
            _ => "Unknown",
        }
    }

    /// Whether this provider type needs an API key field.
    pub fn needs_api_key(&self) -> bool {
        matches!(
            self.provider_type.as_str(),
            "anthropic" | "mistral" | "openai"
        )
    }

    /// Whether this provider type has a base_url field.
    pub fn needs_base_url(&self) -> bool {
        matches!(self.provider_type.as_str(), "ollama" | "openai")
    }
}

/// All editable settings live in a single struct behind one `RwSignal`.
/// Adding a new setting = add a field here + add the UI widget.
#[derive(Clone)]
pub(super) struct SettingsForm {
    pub providers: Vec<ProviderFormEntry>,
    pub next_provider_id: usize,
    // Fonts
    pub markdown_font: FontConfig,
    pub reading_font: FontConfig,
    pub agent_font: FontConfig,
    // Notes
    pub daily_note_folder: String,
    pub daily_note_template_slug: Option<String>,
    // Agent behaviour
    pub max_history: usize,
    pub max_turns: usize,
    pub system_prompt: String,
    pub disabled_tools: Vec<String>,
    pub web_search_api_key: String,
    pub web_search_max_results: usize,
    pub web_fetch_max_output_chars: usize,
    // RAG
    pub rag_enabled: bool,
    pub rag_top_n: usize,
    pub rag_embedding_model: Option<String>,
    // Theme
    pub theme: String,
    // Available tools (loaded async, read-only after init)
    pub available_tools: Vec<ToolInfo>,
    // System fonts (loaded async, read-only after init)
    pub system_fonts: Vec<String>,
}

impl SettingsForm {
    fn from_config(config: &AppConfig) -> Self {
        let providers = config
            .agent
            .providers
            .iter()
            .enumerate()
            .map(|(id, entry)| ProviderFormEntry::from_entry(id, entry))
            .collect();
        Self {
            providers,
            next_provider_id: config.agent.providers.len(),
            markdown_font: config.markdown_font.clone(),
            reading_font: config.reading_font.clone(),
            agent_font: config.agent_font.clone(),
            daily_note_folder: config.daily_note_folder.clone(),
            daily_note_template_slug: config.daily_note_template_slug.clone(),
            max_history: config.agent.max_history,
            max_turns: config.agent.max_turns,
            system_prompt: config
                .agent
                .system_prompt
                .clone()
                .unwrap_or_else(granit_types::default_system_prompt),
            disabled_tools: config.agent.disabled_tools.clone(),
            web_search_api_key: config
                .agent
                .tool_config
                .web_search
                .api_key
                .clone()
                .unwrap_or_default(),
            web_search_max_results: config.agent.tool_config.web_search.max_results,
            web_fetch_max_output_chars: config.agent.tool_config.web_fetch.max_output_chars,
            rag_enabled: config.agent.rag.enabled,
            rag_top_n: config.agent.rag.top_n,
            rag_embedding_model: config.agent.rag.embedding_model.clone(),
            theme: config.theme.clone(),
            available_tools: Vec::new(),
            system_fonts: Vec::new(),
        }
    }
}

/// Settings sections available in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Markdown,
    Reading,
    Agent,
    Notes,
    Theme,
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Reading => "Reading",
            Self::Agent => "Agent",
            Self::Notes => "Notes",
            Self::Theme => "Theme",
        }
    }

    const ALL: [Self; 5] = [
        Self::Markdown,
        Self::Reading,
        Self::Agent,
        Self::Notes,
        Self::Theme,
    ];
}

#[component]
pub fn SettingsModal(set_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let config = ctx.config;
    let active_cave = config.get_untracked().active_cave.unwrap_or_default();
    let cave_name = active_cave
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("Current cave")
        .to_string();
    let modal_title = format!("{cave_name} Settings");
    let modal_subtitle = format!("Saved in {cave_name}/.granit/config.yml");
    // Active section in the sidebar
    let (active_section, set_active_section) = signal(SettingsSection::Agent);

    // All form state in a single signal
    let form = RwSignal::new(SettingsForm::from_config(&config.get_untracked()));
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal(None::<String>);

    // Remember the original theme so we can revert on cancel.
    let original_theme = config.get_untracked().theme.clone();

    // Close without saving: revert theme preview and dismiss modal.
    let cancel = {
        let original_theme = original_theme.clone();
        move || {
            ctx.set_theme(&original_theme);
            set_open.set(false);
        }
    };

    // Load system fonts and available tools on modal open
    leptos::task::spawn_local(async move {
        if let Ok(fonts) = ipc::list_system_fonts().await {
            form.update(|f| f.system_fonts = fonts);
        }
        if let Ok(tools) = ipc::list_tools().await {
            form.update(|f| f.available_tools = tools);
        }
    });

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let f = form.get();
        let existing = config.get_untracked().agent;
        let set_open = set_open;
        set_saving.set(true);
        set_save_error.set(None);
        leptos::task::spawn_local(async move {
            let providers: Vec<ProviderEntry> =
                f.providers.iter().map(|p| p.to_provider_entry()).collect();
            let selected_provider = existing
                .selected_provider
                .min(providers.len().saturating_sub(1));
            let selected_model = if providers == existing.providers
                && selected_provider == existing.selected_provider
            {
                existing.selected_model
            } else {
                None
            };
            let system_prompt = if f.system_prompt.trim().is_empty() {
                None
            } else {
                Some(f.system_prompt)
            };
            let web_search_api_key = if f.web_search_api_key.trim().is_empty() {
                None
            } else {
                Some(f.web_search_api_key)
            };
            let tool_config = ToolsConfig {
                web_search: WebSearchConfig {
                    api_key: web_search_api_key,
                    max_results: f.web_search_max_results,
                },
                web_fetch: WebFetchConfig {
                    max_output_chars: f.web_fetch_max_output_chars,
                },
            };
            let agent = AgentConfig {
                providers,
                selected_provider,
                selected_model,
                mode: config.get_untracked().agent.mode,
                max_history: f.max_history,
                max_turns: f.max_turns,
                system_prompt,
                disabled_tools: f.disabled_tools,
                tool_config,
                rag: RagConfig {
                    enabled: f.rag_enabled,
                    top_n: f.rag_top_n,
                    embedding_model: f.rag_embedding_model,
                },
            };
            let mut next_config = config.get_untracked();
            next_config.agent = agent;
            next_config.markdown_font = f.markdown_font;
            next_config.reading_font = f.reading_font;
            next_config.agent_font = f.agent_font;
            next_config.daily_note_folder = f.daily_note_folder;
            next_config.daily_note_template_slug = f.daily_note_template_slug;
            next_config.theme = f.theme;

            match ipc::save_config(next_config).await {
                Ok(new_config) => {
                    ctx.set_theme(&new_config.theme);
                    config.set(new_config);
                    set_open.set(false);
                }
                Err(e) => set_save_error.set(Some(e)),
            }
            set_saving.set(false);
        });
    };

    let cancel_for_modal = cancel.clone();

    view! {
        <Modal
            title=modal_title
            subtitle=modal_subtitle
            panel_class="w-[640px] max-w-[90vw] h-[480px] max-h-[80vh]"
            on_close=Callback::new(move |()| cancel_for_modal())
        >
            // Body: sidebar + content
            <form class="flex flex-1 min-h-0" on:submit=on_save>
                // Sidebar
                <nav class="w-40 shrink-0 border-r border-base-content/20">
                    <ul class="menu menu-sm py-2">
                        {SettingsSection::ALL.into_iter().map(|section| {
                            let is_active = move || active_section.get() == section;
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class=move || if is_active() { "menu-active" } else { "" }
                                        on:click=move |_| set_active_section.set(section)
                                    >
                                        {section.label()}
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </nav>

                // Content pane
                <div class="flex-1 flex flex-col min-h-0">
                    <div class="flex-1 overflow-y-auto p-4 space-y-4">
                        // Save error
                        <Show when=move || save_error.get().is_some()>
                            <p class="text-xs text-error">
                                {move || save_error.get().unwrap_or_default()}
                            </p>
                        </Show>

                        <Show when=move || active_section.get() == SettingsSection::Markdown>
                            <MarkdownSettings form=form />
                        </Show>

                        <Show when=move || active_section.get() == SettingsSection::Reading>
                            <ReadingSettings form=form />
                        </Show>

                        <Show when=move || active_section.get() == SettingsSection::Agent>
                            <AgentSettings form=form />
                        </Show>

                        <Show when=move || active_section.get() == SettingsSection::Notes>
                            <NotesSettings form=form />
                        </Show>

                        <Show when=move || active_section.get() == SettingsSection::Theme>
                            <ThemeSettings form=form />
                        </Show>
                    </div>

                    // Actions — pinned at bottom
                    <div class="flex justify-end gap-2 px-4 py-3 border-t border-base-content/20 shrink-0">
                        <button
                            type="button"
                            class="btn btn-sm btn-ghost"
                            on:click={
                                let cancel = cancel.clone();
                                move |_| cancel()
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="btn btn-sm btn-primary"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { "Saving\u{2026}" } else { "Save" }}
                        </button>
                    </div>
                </div>
            </form>
        </Modal>
    }
}
