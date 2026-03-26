use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod agent_panel;
mod editor;
mod settings_modal;
mod sidebar;
mod types;

use agent_panel::AgentPanel;
use editor::Editor;
use settings_modal::SettingsModal;
use sidebar::Sidebar;
use types::{AppConfig, Note, NoteMeta};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Fetch the note list from the backend.
pub(crate) async fn fetch_notes() -> Vec<NoteMeta> {
    let result = invoke("list_notes", JsValue::NULL).await;
    serde_wasm_bindgen::from_value(result).unwrap_or_default()
}

#[component]
pub fn App() -> impl IntoView {
    let (sidebar_visible, set_sidebar_visible) = signal(true);
    let (agent_visible, set_agent_visible) = signal(true);
    let (settings_open, set_settings_open) = signal(false);
    let (config, set_config) = signal(AppConfig::default());
    let (notes, set_notes) = signal(Vec::<NoteMeta>::new());
    let (active_note, set_active_note) = signal(None::<Note>);

    // Load config from backend on mount
    let set_config_init = set_config;
    leptos::task::spawn_local(async move {
        let result = invoke("get_config", JsValue::NULL).await;
        if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(result) {
            set_config_init.set(cfg);
        }
    });

    let toggle_sidebar = move |_| set_sidebar_visible.update(|v| *v = !*v);
    let toggle_agent = move |_| set_agent_visible.update(|v| *v = !*v);

    view! {
        <div class="flex flex-col h-screen bg-stone-900 text-stone-200 font-sans">
            // Top bar
            <header class="flex items-center justify-between h-10 px-3 bg-stone-850 border-b border-stone-700 shrink-0">
                <div class="flex items-center gap-2">
                    <button
                        class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                        on:click=toggle_sidebar
                        title="Toggle sidebar"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
                        </svg>
                    </button>
                    <span class="text-sm font-semibold tracking-wide text-stone-300">"Granit"</span>
                </div>
                <button
                    class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                    on:click=toggle_agent
                    title="Toggle agent"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 8.25h9m-9 3H12m-9.75 1.51c0 1.6 1.123 2.994 2.707 3.227 1.129.166 2.27.293 3.423.379.35.026.67.21.865.501L12 21l2.755-4.133a1.14 1.14 0 01.865-.501 48.172 48.172 0 003.423-.379c1.584-.233 2.707-1.626 2.707-3.228V6.741c0-1.602-1.123-2.995-2.707-3.228A48.394 48.394 0 0012 3c-2.392 0-4.744.175-7.043.513C3.373 3.746 2.25 5.14 2.25 6.741v6.018z" />
                    </svg>
                </button>
            </header>

            // Main content area
            <div class="flex flex-1 overflow-hidden">
                // Sidebar (file tree)
                <Show when=move || sidebar_visible.get()>
                    <Sidebar
                        config=config
                        set_config=set_config
                        set_settings_open=set_settings_open
                        notes=notes
                        set_notes=set_notes
                        set_active_note=set_active_note
                    />
                </Show>

                // Editor (center)
                <Editor active_note=active_note set_active_note=set_active_note set_notes=set_notes />

                // Agent panel (right)
                <Show when=move || agent_visible.get()>
                    <AgentPanel />
                </Show>
            </div>

            // Settings modal
            <Show when=move || settings_open.get()>
                <SettingsModal config=config set_config=set_config set_open=set_settings_open />
            </Show>
        </div>
    }
}
