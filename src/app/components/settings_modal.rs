use leptos::prelude::*;

use crate::app::ipc;
use granit_types::{AgentConfig, AppConfig};

#[component]
pub fn SettingsModal(config: RwSignal<AppConfig>, set_open: WriteSignal<bool>) -> impl IntoView {
    // Local form state, initialized from current config
    let (model, set_model) = signal(config.get_untracked().agent.model);
    let (base_url, set_base_url) = signal(
        config.get_untracked().agent.base_url.unwrap_or_default(),
    );
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal(None::<String>);

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let model = model.get();
        let base_url = base_url.get();
        let set_open = set_open;
        set_saving.set(true);
        set_save_error.set(None);
        leptos::task::spawn_local(async move {
            let agent = AgentConfig {
                provider: "ollama".to_string(),
                model: model.clone(),
                base_url: if base_url.trim().is_empty() { None } else { Some(base_url.clone()) },
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
            // Modal panel — stop click propagation so clicking inside doesn't close
            <div
                class="bg-stone-800 border border-stone-600 rounded-lg shadow-xl w-96 max-w-[90vw]"
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

                // Form
                <form class="p-4 space-y-4" on:submit=on_save>
                    // Save error
                    <Show when=move || save_error.get().is_some()>
                        <p class="text-xs text-red-400">
                            {move || save_error.get().unwrap_or_default()}
                        </p>
                    </Show>
                    <fieldset class="space-y-3">
                        <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Agent (Ollama)"</legend>

                        <div class="space-y-1">
                            <label class="block text-xs text-stone-400" for="settings-model">"Model"</label>
                            <input
                                id="settings-model"
                                type="text"
                                class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                                placeholder="qwen3.5:9b"
                                prop:value=move || model.get()
                                on:input=move |ev| set_model.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="space-y-1">
                            <label class="block text-xs text-stone-400" for="settings-base-url">"Base URL"</label>
                            <input
                                id="settings-base-url"
                                type="text"
                                class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                                placeholder="http://localhost:11434"
                                prop:value=move || base_url.get()
                                on:input=move |ev| set_base_url.set(event_target_value(&ev))
                            />
                            <p class="text-xs text-stone-500">"Leave blank to use the default (http://localhost:11434)"</p>
                        </div>
                    </fieldset>

                    // Actions
                    <div class="flex justify-end gap-2 pt-2">
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
                            {move || if saving.get() { "Saving…" } else { "Save" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
