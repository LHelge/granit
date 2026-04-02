mod agent;
mod font_picker;
mod markdown;
mod reading;

use leptos::prelude::*;

use super::icons::XCloseIcon;
use crate::app::ipc;
use crate::app::AppCtx;
use agent::AgentSettings;
use granit_types::{AgentConfig, AppConfig, FontConfig, ProviderConfig, ProviderEntry};
use markdown::MarkdownSettings;
use reading::ReadingSettings;

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
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Reading => "Reading",
            Self::Agent => "Agent",
        }
    }

    const ALL: [Self; 3] = [Self::Markdown, Self::Reading, Self::Agent];
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

    // Load system fonts on modal open
    leptos::task::spawn_local(async move {
        if let Ok(fonts) = ipc::list_system_fonts().await {
            form.update(|f| f.system_fonts = fonts);
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
            let agent = AgentConfig {
                providers,
                selected_provider,
                selected_model: existing.selected_model,
                max_history: existing.max_history,
            };
            match ipc::save_config(agent, f.markdown_font, f.reading_font, f.agent_font).await {
                Ok(new_config) => {
                    config.set(new_config);
                    set_open.set(false);
                }
                Err(e) => set_save_error.set(Some(e)),
            }
            set_saving.set(false);
        });
    };

    let on_backdrop = move |_| set_open.set(false);

    view! {
        // Backdrop
        <div
            class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
            on:click=on_backdrop
        >
            // Modal panel
            <div
                class="bg-stone-800 border border-stone-600 rounded-lg shadow-xl w-[640px] h-[480px] max-w-[90vw] max-h-[80vh] flex flex-col"
                on:click=move |ev| ev.stop_propagation()
            >
                // Header
                <div class="flex items-center justify-between px-4 py-3 border-b border-stone-600">
                    <div>
                        <h2 class="text-sm font-semibold text-stone-200">"Global Settings"</h2>
                        <p class="text-xs text-stone-500 mt-0.5">"Saved to ~/.config/granit/config.yml"</p>
                    </div>
                    <button
                        class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                        on:click=move |_| set_open.set(false)
                    >
                        <XCloseIcon />
                    </button>
                </div>

                // Body: sidebar + content
                <form class="flex flex-1 min-h-0" on:submit=on_save>
                    // Sidebar
                    <nav class="w-40 shrink-0 border-r border-stone-600 py-2">
                        {SettingsSection::ALL.into_iter().map(|section| {
                            let is_active = move || active_section.get() == section;
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        if is_active() {
                                            "w-full text-left px-4 py-1.5 text-sm text-stone-200 bg-stone-700"
                                        } else {
                                            "w-full text-left px-4 py-1.5 text-sm text-stone-400 hover:text-stone-200 hover:bg-stone-700/50 transition-colors"
                                        }
                                    }
                                    on:click=move |_| set_active_section.set(section)
                                >
                                    {section.label()}
                                </button>
                            }
                        }).collect_view()}
                    </nav>

                    // Content pane
                    <div class="flex-1 flex flex-col min-h-0">
                        <div class="flex-1 overflow-y-auto p-4 space-y-4">
                            // Save error
                            <Show when=move || save_error.get().is_some()>
                                <p class="text-xs text-red-400">
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
                        </div>

                        // Actions — pinned at bottom
                        <div class="flex justify-end gap-2 px-4 py-3 border-t border-stone-600">
                            <button
                                type="button"
                                class="px-3 py-1.5 text-sm rounded border border-stone-600 text-stone-300 hover:bg-stone-700 transition-colors"
                                on:click=move |_| set_open.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                type="submit"
                                class="px-3 py-1.5 text-sm rounded bg-stone-600 text-stone-200 hover:bg-stone-500 transition-colors disabled:opacity-50"
                                disabled=move || saving.get()
                            >
                                {move || if saving.get() { "Saving\u{2026}" } else { "Save" }}
                            </button>
                        </div>
                    </div>
                </form>
            </div>
        </div>
    }
}
