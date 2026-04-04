mod agent;
mod font_picker;
mod markdown;
mod notes;
mod reading;
mod theme;

use super::icons::Icon;
use crate::app::{ipc, AppCtx};
use agent::AgentSettings;
use granit_types::{AgentConfig, AppConfig, FontConfig, ProviderConfig, ProviderEntry, ToolInfo};
use leptos::prelude::*;
use markdown::MarkdownSettings;
use notes::NotesSettings;
use reading::ReadingSettings;
use theme::ThemeSettings;

/// Flat representation of one provider for form editing.
#[derive(Clone)]
pub(super) struct ProviderFormEntry {
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl ProviderFormEntry {
    fn from_entry(entry: &ProviderEntry) -> Self {
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
            ProviderConfig::Prisma { api_key } => ("prisma".into(), String::new(), api_key.clone()),
        };
        Self {
            provider_type,
            name: entry.name.clone().unwrap_or_default(),
            base_url,
            api_key,
        }
    }

    fn new_default(provider_type: &str) -> Self {
        Self {
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
            "prisma" => ProviderConfig::Prisma {
                api_key: self.api_key.clone(),
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
            "prisma" => "Prisma",
            _ => "Unknown",
        }
    }

    /// Whether this provider type needs an API key field.
    pub fn needs_api_key(&self) -> bool {
        matches!(
            self.provider_type.as_str(),
            "anthropic" | "mistral" | "prisma"
        )
    }

    /// Whether this provider type has a base_url field.
    pub fn needs_base_url(&self) -> bool {
        self.provider_type == "ollama"
    }
}

/// All editable settings live in a single struct behind one `RwSignal`.
/// Adding a new setting = add a field here + add the UI widget.
#[derive(Clone)]
pub(super) struct SettingsForm {
    pub providers: Vec<ProviderFormEntry>,
    // Fonts
    pub markdown_font: FontConfig,
    pub reading_font: FontConfig,
    pub agent_font: FontConfig,
    // Notes
    pub daily_note_folder: String,
    // Agent behaviour
    pub max_history: usize,
    pub max_turns: usize,
    pub system_prompt: String,
    pub disabled_tools: Vec<String>,
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
            .map(ProviderFormEntry::from_entry)
            .collect();
        Self {
            providers,
            markdown_font: config.markdown_font.clone(),
            reading_font: config.reading_font.clone(),
            agent_font: config.agent_font.clone(),
            daily_note_folder: config.daily_note_folder.clone(),
            max_history: config.agent.max_history,
            max_turns: config.agent.max_turns,
            system_prompt: config
                .agent
                .system_prompt
                .clone()
                .unwrap_or_else(granit_types::default_system_prompt),
            disabled_tools: config.agent.disabled_tools.clone(),
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
    let config = expect_context::<AppCtx>().config;
    // Active section in the sidebar
    let (active_section, set_active_section) = signal(SettingsSection::Agent);

    // All form state in a single signal
    let form = RwSignal::new(SettingsForm::from_config(&config.get_untracked()));
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal(None::<String>);

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
            let system_prompt = if f.system_prompt.trim().is_empty() {
                None
            } else {
                Some(f.system_prompt)
            };
            let agent = AgentConfig {
                providers,
                selected_provider,
                selected_model: existing.selected_model,
                max_history: f.max_history,
                max_turns: f.max_turns,
                system_prompt,
                disabled_tools: f.disabled_tools,
            };
            match ipc::save_config(
                agent,
                f.markdown_font,
                f.reading_font,
                f.agent_font,
                f.daily_note_folder,
            )
            .await
            {
                Ok(new_config) => {
                    config.set(new_config);
                    set_open.set(false);
                }
                Err(e) => set_save_error.set(Some(e)),
            }
            set_saving.set(false);
        });
    };

    view! {
        <dialog class="modal modal-open">
            // Modal panel
            <div class="modal-box w-[640px] max-w-[90vw] h-[480px] max-h-[80vh] p-0 flex flex-col">
                // Header
                <div class="flex items-center justify-between px-4 py-3 border-b border-base-content/20 shrink-0">
                    <div>
                        <h2 class="text-sm font-semibold text-base-content">"Global Settings"</h2>
                        <p class="text-xs text-base-content/35 mt-0.5">"Saved to ~/.config/granit/config.yml"</p>
                    </div>
                    <button
                        class="btn btn-ghost btn-xs btn-square"
                        on:click=move |_| set_open.set(false)
                    >
                        <Icon icon=icondata_lu::LuX width="1rem" height="1rem"/>
                    </button>
                </div>

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
                                <ThemeSettings />
                            </Show>
                        </div>

                        // Actions — pinned at bottom
                        <div class="flex justify-end gap-2 px-4 py-3 border-t border-base-content/20 shrink-0">
                            <button
                                type="button"
                                class="btn btn-sm btn-ghost"
                                on:click=move |_| set_open.set(false)
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
            </div>
            // Backdrop — click to close
            <form method="dialog" class="modal-backdrop">
                <button on:click=move |_| set_open.set(false)>"close"</button>
            </form>
        </dialog>
    }
}
