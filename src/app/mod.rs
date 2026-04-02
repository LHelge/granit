use leptos::prelude::*;

mod components;
pub(crate) mod ipc;

use components::editor::EditOpen;
use components::icons::{PanelLeftIcon, PanelRightIcon};
use components::{AgentPanel, Editor, OpenInEdit, SettingsModal, Sidebar};
use granit_types::{AppConfig, Note, NoteMeta};

// ── App-wide shared state via context ──────────────────────────────

/// A single error entry in the unified error channel.
#[derive(Clone, PartialEq)]
pub struct AppError {
    id: u32,
    pub source: &'static str,
    pub message: String,
}

/// Shared reactive state provided via Leptos context so child components can
/// `expect_context::<AppCtx>()` instead of receiving these signals as props.
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub config: RwSignal<AppConfig>,
    pub notes: RwSignal<Vec<NoteMeta>>,
    pub folders: RwSignal<Vec<String>>,
    pub active_note: RwSignal<Option<Note>>,
    pub is_mac: bool,
    errors: RwSignal<Vec<AppError>>,
    next_id: RwSignal<u32>,
}

impl AppCtx {
    /// Push an error and return its id (for later dismissal).
    pub fn push_error(&self, source: &'static str, message: impl Into<String>) -> u32 {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        self.errors.update(|v| {
            v.push(AppError {
                id,
                source,
                message: message.into(),
            })
        });
        // Auto-dismiss after 8 seconds
        let ctx = *self;
        leptos::task::spawn_local(async move {
            gloo_timers::future::sleep(std::time::Duration::from_secs(8)).await;
            ctx.dismiss(id);
        });
        id
    }

    /// Dismiss a single error by id.
    pub fn dismiss(&self, id: u32) {
        self.errors.update(|v| v.retain(|e| e.id != id));
    }

    /// Remove all errors from a given source.
    pub fn clear_source(&self, source: &'static str) {
        self.errors.update(|v| v.retain(|e| e.source != source));
    }

    /// Get the first error for a source (for inline display).
    pub fn first_error_for(&self, source: &'static str) -> Option<String> {
        self.errors
            .get()
            .iter()
            .find(|e| e.source == source)
            .map(|e| e.message.clone())
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (sidebar_visible, set_sidebar_visible) = signal(true);
    let (agent_visible, set_agent_visible) = signal(false);
    let (settings_open, set_settings_open) = signal(false);

    let is_mac = web_sys::window()
        .and_then(|w| w.navigator().platform().ok())
        .map(|p: String| p.contains("Mac"))
        .unwrap_or(false);

    let ctx = AppCtx {
        config: RwSignal::new(AppConfig::default()),
        notes: RwSignal::new(Vec::<NoteMeta>::new()),
        folders: RwSignal::new(Vec::<String>::new()),
        active_note: RwSignal::new(None::<Note>),
        is_mac,
        errors: RwSignal::new(Vec::new()),
        next_id: RwSignal::new(0),
    };
    provide_context(ctx);
    provide_context(OpenInEdit(RwSignal::new(EditOpen::Preview)));

    // Sync active_note changes to the backend so agent tools can see it.
    Effect::new(move |_| {
        let slug = ctx.active_note.get().map(|n| n.meta.slug.clone());
        leptos::task::spawn_local(async move {
            let _ = ipc::set_active_note(slug.as_deref()).await;
        });
    });

    // Listen for cave mutations (from agent tools or other sources) and
    // refresh notes, folders, and the active note. Registered at the app
    // root so the listener is always alive regardless of panel visibility.
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let _handle = ipc::listen_event_simple("cave:notes-changed", move || {
                leptos::task::spawn_local(async move {
                    if let Ok(notes) = ipc::fetch_notes().await {
                        if let Some(active) = ctx.active_note.get_untracked() {
                            if !notes.iter().any(|n| n.slug == active.meta.slug) {
                                ctx.active_note.set(None);
                            } else if let Ok(note) = ipc::read_note(&active.meta.slug).await {
                                ctx.active_note.set(Some(note));
                            }
                        }
                        ctx.notes.set(notes);
                    }
                    if let Ok(folders) = ipc::fetch_folders().await {
                        ctx.folders.set(folders);
                    }
                });
            })
            .await;

            // Keep handle alive forever (app root never unmounts).
            std::future::pending::<()>().await;
        });
    });

    // Load config from backend on mount, and re-open the most recent cave if any
    leptos::task::spawn_local(async move {
        let cfg = match ipc::fetch_config().await {
            Ok(c) => c,
            Err(e) => {
                ctx.push_error("config", format!("Failed to load config: {e}"));
                return;
            }
        };
        let recent = cfg.recent_caves.first().cloned();
        ctx.config.set(cfg);

        // Re-open the last cave so the backend has a cave_path set
        if let Some(path) = recent {
            match ipc::open_cave(&path).await {
                Ok(new_cfg) => {
                    ctx.config.set(new_cfg);
                    match ipc::fetch_notes().await {
                        Ok(n) => {
                            ctx.clear_source("notes");
                            ctx.notes.set(n);
                        }
                        Err(e) => {
                            ctx.clear_source("notes");
                            ctx.push_error("notes", e);
                        }
                    }
                    if let Ok(f) = ipc::fetch_folders().await {
                        ctx.folders.set(f);
                    }
                }
                Err(e) => {
                    ctx.push_error("cave", format!("Failed to reopen cave: {e}"));
                }
            }
        }
    });

    let toggle_sidebar = move |_| set_sidebar_visible.update(|v| *v = !*v);
    let toggle_agent = move |_| set_agent_visible.update(|v| *v = !*v);

    // macOS needs extra left margin for traffic-light window buttons
    let title_margin = if ctx.is_mac { "ml-16" } else { "ml-2" };

    view! {
        <div class="flex flex-col h-screen bg-stone-900 text-stone-200 font-sans">
            // Top bar
            <header data-tauri-drag-region class="titlebar flex items-center justify-between h-8 px-3 bg-stone-850 border-b border-stone-700 shrink-0">
                <span class=format!("text-sm font-semibold tracking-wide text-stone-300 mt-1 {title_margin}")>"Granit"</span>
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
                    <Sidebar set_settings_open=set_settings_open />
                </Show>

                // Editor (center)
                <Editor />

                // Agent panel (right)
                <Show when=move || agent_visible.get()>
                    <AgentPanel />
                </Show>
            </div>

            // Settings modal
            <Show when=move || settings_open.get()>
                <SettingsModal set_open=set_settings_open />
            </Show>

            // Toast notifications (bottom-right)
            <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none">
                <For
                    each=move || ctx.errors.get()
                    key=|e| e.id
                    let:err
                >
                    <div class="pointer-events-auto flex items-start gap-2 px-3 py-2.5 rounded-lg shadow-lg bg-red-950/90 border border-red-800/60 text-red-200 text-xs backdrop-blur-sm animate-[toast-in_0.2s_ease-out]">
                        <span class="flex-1 leading-relaxed">{err.message.clone()}</span>
                        <button
                            class="mt-0.5 text-red-400 hover:text-red-200 shrink-0"
                            on:click={
                                let id = err.id;
                                move |_| ctx.dismiss(id)
                            }
                        >
                            "✕"
                        </button>
                    </div>
                </For>
            </div>
        </div>
    }
}
