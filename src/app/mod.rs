use leptos::prelude::*;

mod components;
pub(crate) mod ipc;

use components::icons::{PanelLeftIcon, PanelRightIcon};
use components::{AgentPanel, Editor, OpenInEdit, SettingsModal, Sidebar};
use components::editor::EditOpen;
use granit_types::{AppConfig, Note, NoteMeta};

#[component]
pub fn App() -> impl IntoView {
    let (sidebar_visible, set_sidebar_visible) = signal(true);
    let (agent_visible, set_agent_visible) = signal(true);
    let (settings_open, set_settings_open) = signal(false);
    let config = RwSignal::new(AppConfig::default());
    let notes = RwSignal::new(Vec::<NoteMeta>::new());
    let active_note = RwSignal::new(None::<Note>);
    let error_msg = RwSignal::new(None::<String>);
    let notes_error = RwSignal::new(None::<String>);

    provide_context(OpenInEdit(RwSignal::new(EditOpen::Preview)));

    // Load config from backend on mount, and re-open the most recent cave if any
    leptos::task::spawn_local(async move {
        let cfg = match ipc::fetch_config().await {
            Ok(c) => c,
            Err(e) => {
                error_msg.set(Some(format!("Failed to load config: {e}")));
                return;
            }
        };
        let recent = cfg.recent_caves.first().cloned();
        config.set(cfg);

        // Re-open the last cave so the backend has a cave_path set
        if let Some(path) = recent {
            match ipc::open_cave(&path).await {
                Ok(new_cfg) => {
                    config.set(new_cfg);
                    match ipc::fetch_notes().await {
                        Ok(n) => {
                            notes_error.set(None);
                            notes.set(n);
                        }
                        Err(e) => notes_error.set(Some(e)),
                    }
                }
                Err(e) => error_msg.set(Some(format!("Failed to reopen cave: {e}"))),
            }
        }
    });

    let toggle_sidebar = move |_| set_sidebar_visible.update(|v| *v = !*v);
    let toggle_agent = move |_| set_agent_visible.update(|v| *v = !*v);

    view! {
        <div class="flex flex-col h-screen bg-stone-900 text-stone-200 font-sans">
            // Global error banner
            <Show when=move || error_msg.get().is_some()>
                <div class="px-3 py-1.5 bg-red-900/70 border-b border-red-700 text-red-300 text-xs flex items-center gap-2">
                    <span class="flex-1">{move || error_msg.get().unwrap_or_default()}</span>
                    <button
                        class="text-red-400 hover:text-red-200"
                        on:click=move |_| error_msg.set(None)
                    >
                        "✕"
                    </button>
                </div>
            </Show>
            // Top bar
            <header data-tauri-drag-region class="titlebar flex items-center justify-between h-8 px-3 bg-stone-850 border-b border-stone-700 shrink-0">
                <span class="text-sm font-semibold tracking-wide text-stone-300 ml-16 mt-1">"Granit"</span>
                <div class="flex items-center gap-1">
                    <button
                        class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                        on:click=toggle_sidebar
                        title="Toggle sidebar"
                    >
                        <PanelLeftIcon />
                    </button>
                    <button
                        class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                        on:click=toggle_agent
                        title="Toggle agent"
                    >
                        <PanelRightIcon />
                    </button>
                </div>
            </header>

            // Main content area
            <div class="flex flex-1 overflow-hidden">
                // Sidebar (file tree)
                <Show when=move || sidebar_visible.get()>
                    <Sidebar
                        config=config
                        set_settings_open=set_settings_open
                        notes=notes
                        active_note=active_note
                        error_msg=error_msg
                        notes_error=notes_error
                    />
                </Show>

                // Editor (center)
                <Editor active_note=active_note notes=notes config=config />

                // Agent panel (right)
                <Show when=move || agent_visible.get()>
                    <AgentPanel
                        config=config
                    />
                </Show>
            </div>

            // Settings modal
            <Show when=move || settings_open.get()>
                <SettingsModal config=config set_open=set_settings_open />
            </Show>
        </div>
    }
}
