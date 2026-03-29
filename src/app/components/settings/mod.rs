mod agent;
mod markdown;
mod reading;

use leptos::prelude::*;

use crate::app::ipc;
use agent::AgentSettings;
use granit_types::{AgentConfig, AppConfig};
use markdown::MarkdownSettings;
use reading::ReadingSettings;

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
pub fn SettingsModal(config: RwSignal<AppConfig>, set_open: WriteSignal<bool>) -> impl IntoView {
    // Active section in the sidebar
    let (active_section, set_active_section) = signal(SettingsSection::Agent);

    // Agent form state, initialized from current config
    let (provider, set_provider) = signal(config.get_untracked().agent.provider);
    let (model, set_model) = signal(config.get_untracked().agent.model);
    let (base_url, set_base_url) =
        signal(config.get_untracked().agent.base_url.unwrap_or_default());
    let (api_key, set_api_key) = signal(String::new());
    let (api_key_is_set, set_api_key_is_set) = signal(false);
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal(None::<String>);

    // Check if Anthropic API key is configured on modal open
    leptos::task::spawn_local(async move {
        if let Ok(Some(true)) = ipc::get_secret("ANTHROPIC_API_KEY").await {
            set_api_key_is_set.set(true);
        }
    });

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let provider = provider.get();
        let model = model.get();
        let base_url = base_url.get();
        let api_key_val = api_key.get();
        let set_open = set_open;
        set_saving.set(true);
        set_save_error.set(None);
        leptos::task::spawn_local(async move {
            // Save API key if the user entered a new one
            if !api_key_val.is_empty() {
                if let Err(e) = ipc::set_secret("ANTHROPIC_API_KEY", &api_key_val).await {
                    set_save_error.set(Some(e));
                    set_saving.set(false);
                    return;
                }
            }

            let agent = AgentConfig {
                provider: provider.clone(),
                model: model.clone(),
                base_url: if base_url.trim().is_empty() {
                    None
                } else {
                    Some(base_url.clone())
                },
            };
            match ipc::save_config(agent).await {
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
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                        </svg>
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
                                <MarkdownSettings />
                            </Show>

                            <Show when=move || active_section.get() == SettingsSection::Reading>
                                <ReadingSettings />
                            </Show>

                            <Show when=move || active_section.get() == SettingsSection::Agent>
                                <AgentSettings
                                    provider=provider
                                    set_provider=set_provider
                                    model=model
                                    set_model=set_model
                                    base_url=base_url
                                    set_base_url=set_base_url
                                    api_key=api_key
                                    set_api_key=set_api_key
                                    api_key_is_set=api_key_is_set
                                />
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
