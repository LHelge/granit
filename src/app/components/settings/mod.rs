mod agent;
mod font_picker;
mod markdown;
mod reading;

use leptos::prelude::*;

use super::icons::XCloseIcon;
use crate::app::ipc;
use crate::app::AppCtx;
use agent::AgentSettings;
use granit_types::{AgentConfig, AppConfig, FontConfig};
use markdown::MarkdownSettings;
use reading::ReadingSettings;

/// Map a provider name to its secrets.env key, if it requires an API key.
fn api_key_name(provider: &str) -> Option<&'static str> {
    match provider {
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "mistral" => Some("MISTRAL_API_KEY"),
        "prisma" => Some("PRISMA_API_KEY"),
        _ => None,
    }
}

/// All editable settings live in a single struct behind one `RwSignal`.
/// Adding a new setting = add a field here + add the UI widget.
#[derive(Clone)]
pub(super) struct SettingsForm {
    // Agent
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub api_key_is_set: bool,
    // Fonts
    pub markdown_font: FontConfig,
    pub reading_font: FontConfig,
    pub agent_font: FontConfig,
    // System fonts (loaded async, read-only after init)
    pub system_fonts: Vec<String>,
}

impl SettingsForm {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            provider: config.agent.provider.clone(),
            model: config.agent.model.clone(),
            base_url: config.agent.base_url.clone().unwrap_or_default(),
            api_key: String::new(),
            api_key_is_set: false,
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

    // Check if API key is configured for the current provider on modal open
    leptos::task::spawn_local(async move {
        let current_provider = form.get_untracked().provider;
        let secret_key = api_key_name(&current_provider);
        if let Some(key) = secret_key {
            if let Ok(Some(true)) = ipc::get_secret(key).await {
                form.update(|f| f.api_key_is_set = true);
            }
        }
        if let Ok(fonts) = ipc::list_system_fonts().await {
            form.update(|f| f.system_fonts = fonts);
        }
    });

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let f = form.get();
        let max_history = config.get_untracked().agent.max_history;
        let set_open = set_open;
        set_saving.set(true);
        set_save_error.set(None);
        leptos::task::spawn_local(async move {
            // Save API key if the user entered a new one
            if !f.api_key.is_empty() {
                if let Some(key_name) = api_key_name(&f.provider) {
                    if let Err(e) = ipc::set_secret(key_name, &f.api_key).await {
                        set_save_error.set(Some(e));
                        set_saving.set(false);
                        return;
                    }
                }
            }

            let agent = AgentConfig {
                provider: f.provider,
                model: f.model,
                base_url: if f.base_url.trim().is_empty() {
                    None
                } else {
                    Some(f.base_url)
                },
                max_history,
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
