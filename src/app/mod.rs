mod agent;
mod components;
pub(crate) mod context;
mod editor;
mod explorer;
mod info;
pub(crate) mod ipc;
mod markdown_links;
mod settings;

pub(crate) use context::AppCtx;
use leptos::prelude::*;

use agent::AgentPanel;
use components::icons::Icon;
use editor::{EditOpen, Editor, OpenInEdit};
use explorer::Explorer;
use granit_types::SidebarConfig;
use info::InfoModal;
use settings::SettingsModal;

// ── Sidebar resize constants ───────────────────────────────────────

const MIN_SIDEBAR_W: u16 = 275;
const MAX_SIDEBAR_W: u16 = 600;

#[derive(Clone, Copy)]
enum ResizeTarget {
    Sidebar,
    Agent,
}

#[component]
pub fn App() -> impl IntoView {
    let (sidebar_visible, set_sidebar_visible) = signal(true);
    let (sidebar_width, set_sidebar_width) = signal(256u16);
    let (agent_visible, set_agent_visible) = signal(false);
    let (agent_width, set_agent_width) = signal(320u16);
    let (info_open, set_info_open) = signal(false);
    let (settings_open, set_settings_open) = signal(false);

    let is_mac = web_sys::window()
        .and_then(|w| w.navigator().platform().ok())
        .map(|p: String| p.contains("Mac"))
        .unwrap_or(false);

    let ctx = AppCtx::new(is_mac);
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
        ipc::spawn_event_listener_simple(granit_types::CAVE_NOTES_CHANGED, move || {
            leptos::task::spawn_local(async move {
                if let Ok(notes) = ipc::fetch_notes().await {
                    if let Some(active) = ctx.active_note.get_untracked() {
                        if !notes.iter().any(|n| n.slug == active.meta.slug) {
                            ctx.clear_active_document();
                        } else if let Ok(note) = ipc::read_note(&active.meta.slug).await {
                            ctx.set_active_note_document(note);
                        }
                    }
                    ctx.notes.set(notes);
                }
                if let Ok(folders) = ipc::fetch_folders().await {
                    ctx.folders.set(folders);
                }
            });
        });
    });

    // Load config from backend on mount.
    leptos::task::spawn_local(async move {
        let cfg = match ipc::fetch_config().await {
            Ok(c) => c,
            Err(e) => {
                ctx.push_error("config", format!("Failed to load config: {e}"));
                return;
            }
        };
        let has_active_cave = cfg.active_cave.is_some();
        // Apply persisted sidebar state
        set_sidebar_visible.set(cfg.sidebar.visible);
        set_sidebar_width.set(cfg.sidebar.width);
        set_agent_visible.set(cfg.agent_panel.visible);
        set_agent_width.set(cfg.agent_panel.width);
        let theme_name = cfg.theme.clone();
        ctx.config.set(cfg);

        // Apply the persisted theme immediately to avoid a flash of the default theme
        ctx.set_theme(&theme_name);

        if has_active_cave {
            match ipc::fetch_notes().await {
                Ok(notes) => ctx.notes.set(notes),
                Err(e) => {
                    ctx.push_error("notes", format!("Failed to load restored cave: {e}"));
                }
            }

            match ipc::fetch_folders().await {
                Ok(folders) => ctx.folders.set(folders),
                Err(e) => {
                    ctx.push_error("folders", format!("Failed to load folders: {e}"));
                }
            }

            match ipc::fetch_templates().await {
                Ok(templates) => ctx.templates.set(templates),
                Err(e) => {
                    ctx.push_error("templates", format!("Failed to load templates: {e}"));
                }
            }
        }
    });

    // Persist sidebar visibility / width to the backend config.
    let persist_sidebar_state = move || {
        let sb = SidebarConfig {
            visible: sidebar_visible.get_untracked(),
            width: sidebar_width.get_untracked(),
        };
        let ap = SidebarConfig {
            visible: agent_visible.get_untracked(),
            width: agent_width.get_untracked(),
        };
        leptos::task::spawn_local(async move {
            let _ = ipc::save_sidebar_state(sb, ap).await;
        });
    };

    let toggle_sidebar = move |_| {
        set_sidebar_visible.update(|v| *v = !*v);
        persist_sidebar_state();
    };
    let toggle_agent = move |_| {
        set_agent_visible.update(|v| *v = !*v);
        persist_sidebar_state();
    };

    // macOS needs extra left margin for traffic-light window buttons
    let title_margin = if ctx.is_mac { "ml-16" } else { "ml-2" };

    // ── Resize logic ───────────────────────────────────────────────
    // Which panel is being resized (if any).
    let (resizing, set_resizing) = signal(None::<ResizeTarget>);

    let on_global_mousemove = move |ev: web_sys::MouseEvent| {
        let Some(target) = resizing.get_untracked() else {
            return;
        };
        let x = ev.client_x() as u16;
        match target {
            ResizeTarget::Sidebar => {
                let w = x.clamp(MIN_SIDEBAR_W, MAX_SIDEBAR_W);
                set_sidebar_width.set(w);
            }
            ResizeTarget::Agent => {
                // Agent width = viewport width - mouse x
                let vw = web_sys::window()
                    .and_then(|w| w.inner_width().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1024.0) as u16;
                let w = vw.saturating_sub(x).clamp(MIN_SIDEBAR_W, MAX_SIDEBAR_W);
                set_agent_width.set(w);
            }
        }
    };

    let on_global_mouseup = move |_: web_sys::MouseEvent| {
        if resizing.get_untracked().is_some() {
            set_resizing.set(None);
            persist_sidebar_state();
        }
    };

    let start_sidebar_resize = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_resizing.set(Some(ResizeTarget::Sidebar));
    };
    let start_agent_resize = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_resizing.set(Some(ResizeTarget::Agent));
    };

    // Cursor style: show col-resize while dragging
    let resize_cursor = move || {
        if resizing.get().is_some() {
            "cursor-col-resize select-none"
        } else {
            ""
        }
    };

    view! {
        <div
            class=move || format!("flex flex-col h-screen bg-base-100 text-base-content font-sans {}", resize_cursor())
            on:mousemove=on_global_mousemove
            on:mouseup=on_global_mouseup
        >
            // Top bar
            <header data-tauri-drag-region class="titlebar flex items-center justify-between h-8 px-3 bg-base-200 border-b border-base-content/10 shrink-0">
                <div class="flex items-center gap-1">
                    <span class=format!("text-sm font-semibold tracking-wide text-base-content/70 mt-1 {title_margin}")>"Granit"</span>
                </div>
                <div class="flex items-center gap-1">
                    <div class="tooltip tooltip-left" data-tip="Toggle sidebar">
                        <button
                            class="btn btn-ghost btn-xs btn-square"
                            on:click=toggle_sidebar
                        >
                            <Icon icon=icondata_lu::LuPanelLeft width="1rem" height="1rem"/>
                        </button>
                    </div>
                    <div class="tooltip tooltip-left" data-tip="Toggle agent">
                        <button
                            class="btn btn-ghost btn-xs btn-square"
                            on:click=toggle_agent
                        >
                            <Icon icon=icondata_lu::LuPanelRight width="1rem" height="1rem"/>
                        </button>
                    </div>
                </div>
            </header>

            // Main content area
            <div class="flex flex-1 overflow-hidden">
                // Explorer (file tree)
                <Show when=move || sidebar_visible.get()>
                    <Explorer
                        set_settings_open=set_settings_open
                        set_info_open=set_info_open
                        width=sidebar_width
                    />
                    // Resize handle
                    <div
                        class="w-1 shrink-0 cursor-col-resize hover:bg-base-content/30 active:bg-primary transition-colors"
                        on:mousedown=start_sidebar_resize
                    />
                </Show>

                // Editor (center)
                <Editor />

                // Agent panel (right)
                <Show when=move || agent_visible.get()>
                    // Resize handle
                    <div
                        class="w-1 shrink-0 cursor-col-resize hover:bg-base-content/30 active:bg-primary transition-colors"
                        on:mousedown=start_agent_resize
                    />
                    <AgentPanel width=agent_width />
                </Show>
            </div>

            // Settings modal
            <Show when=move || settings_open.get() && ctx.config.get().active_cave.is_some()>
                <SettingsModal set_open=set_settings_open />
            </Show>

            <Show when=move || info_open.get()>
                <InfoModal set_open=set_info_open />
            </Show>

            // Toast notifications (bottom-right)
            <div class="toast toast-end toast-bottom z-50">
                <For
                    each=move || ctx.errors().get()
                    key=|e| e.id
                    let:err
                >
                    <div role="alert" class="alert alert-error alert-soft alert-sm shadow-lg max-w-sm">
                        <span class="flex-1 text-xs leading-relaxed">{err.message.clone()}</span>
                        <button
                            class="btn btn-ghost btn-xs btn-square"
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
