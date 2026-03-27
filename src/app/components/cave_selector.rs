use leptos::prelude::*;

use crate::app::ipc;
use crate::app::types::{AppConfig, Note, NoteMeta};

#[component]
pub fn CaveSelector(
    config: ReadSignal<AppConfig>,
    set_config: WriteSignal<AppConfig>,
    set_notes: WriteSignal<Vec<NoteMeta>>,
    set_active_note: WriteSignal<Option<Note>>,
    set_settings_open: WriteSignal<bool>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let (dropdown_open, set_dropdown_open) = signal(false);

    let open_and_refresh = move |path: String| {
        leptos::task::spawn_local(async move {
            match ipc::open_cave(&path).await {
                Ok(new_config) => {
                    set_config.set(new_config);
                    match ipc::fetch_notes().await {
                        Ok(n) => {
                            notes_error.set(None);
                            set_notes.set(n);
                        }
                        Err(e) => notes_error.set(Some(e)),
                    }
                    set_active_note.set(None);
                }
                Err(e) => error_msg.set(Some(format!("Failed to open cave: {e}"))),
            }
            set_dropdown_open.set(false);
        });
    };

    let on_pick_folder = move |_| {
        leptos::task::spawn_local(async move {
            if let Some(path) = ipc::pick_folder().await {
                open_and_refresh(path);
            } else {
                set_dropdown_open.set(false);
            }
        });
    };

    let cave_label = move || {
        let cfg = config.get();
        cfg.active_cave
            .as_deref()
            .and_then(|p| p.rsplit('/').next().or_else(|| p.rsplit('\\').next()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "No cave open".to_string())
    };

    view! {
        <div class="border-t border-stone-700 px-2 py-2">
            <div class="flex items-center gap-1">
                // Cave selector dropdown
                <div class="relative flex-1">
                    <button
                        class="w-full flex items-center justify-between px-2 py-1.5 text-sm bg-stone-800 border border-stone-600 rounded hover:border-stone-500 transition-colors text-stone-300 text-left truncate"
                        on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
                    >
                        <span class="truncate">{cave_label}</span>
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 ml-1 shrink-0 text-stone-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                        </svg>
                    </button>

                    // Dropdown menu (opens upward)
                    <Show when=move || dropdown_open.get()>
                        <div class="absolute bottom-full left-0 right-0 mb-1 bg-stone-800 border border-stone-600 rounded shadow-lg z-50 max-h-60 overflow-y-auto">
                            // Recent caves
                            {move || {
                                let cfg = config.get();
                                cfg.recent_caves.iter().map(|path| {
                                    let path_clone = path.clone();
                                    let display = path.rsplit('/').next()
                                        .or_else(|| path.rsplit('\\').next())
                                        .unwrap_or(path)
                                        .to_string();
                                    let full_path = path.clone();
                                    view! {
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700 transition-colors truncate"
                                            title=full_path
                                            on:click=move |_| open_and_refresh(path_clone.clone())
                                        >
                                            {display}
                                        </button>
                                    }
                                }).collect_view()
                            }}

                            // Divider
                            <div class="border-t border-stone-600 my-1"></div>

                            // Single "Open folder…" button (replaces duplicate open/create)
                            <button
                                class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700 transition-colors"
                                on:click=on_pick_folder
                            >
                                "Open folder…"
                            </button>
                        </div>
                    </Show>
                </div>

                // Settings gear icon
                <button
                    class="p-1.5 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                    title="Settings"
                    on:click=move |_| set_settings_open.set(true)
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28z" />
                        <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                </button>
            </div>
        </div>
    }
}
