use leptos::prelude::*;

use crate::app::ipc;
use crate::app::types::{AgentConfig, AppConfig};

#[component]
pub fn SettingsModal(
    config: ReadSignal<AppConfig>,
    set_config: WriteSignal<AppConfig>,
    set_open: WriteSignal<bool>,
) -> impl IntoView {
    // Local form state, initialized from current config
    let (provider, set_provider) = signal(config.get_untracked().agent.provider);
    let (model, set_model) = signal(config.get_untracked().agent.model);
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal(None::<String>);

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let provider = provider.get();
        let model = model.get();
        let set_config = set_config;
        let set_open = set_open;
        set_saving.set(true);
        set_save_error.set(None);
        leptos::task::spawn_local(async move {
            let agent = AgentConfig {
                provider: provider.clone(),
                model: model.clone(),
            };
            match ipc::save_config(agent).await {
                Ok(new_config) => {
                    set_config.set(new_config);
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
                        <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Agent"</legend>

                        <div class="space-y-1">
                            <label class="block text-xs text-stone-400" for="settings-provider">"Provider"</label>
                            <input
                                id="settings-provider"
                                type="text"
                                class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                                placeholder="openai"
                                prop:value=move || provider.get()
                                on:input=move |ev| set_provider.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="space-y-1">
                            <label class="block text-xs text-stone-400" for="settings-model">"Model"</label>
                            <input
                                id="settings-model"
                                type="text"
                                class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                                placeholder="gpt-4o"
                                prop:value=move || model.get()
                                on:input=move |ev| set_model.set(event_target_value(&ev))
                            />
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
