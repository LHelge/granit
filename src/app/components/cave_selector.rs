use leptos::prelude::*;

use super::icons::{ChevronDownIcon, GearIcon};
use crate::app::ipc;
use crate::app::AppCtx;

#[component]
pub fn CaveSelector(set_settings_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let (dropdown_open, set_dropdown_open) = signal(false);

    let open_and_refresh = move |path: String| {
        leptos::task::spawn_local(async move {
            match ipc::open_cave(&path).await {
                Ok(new_config) => {
                    ctx.config.set(new_config);
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
                    ctx.active_note.set(None);
                }
                Err(e) => {
                    ctx.push_error("cave", format!("Failed to open cave: {e}"));
                }
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
        let cfg = ctx.config.get();
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
                        <ChevronDownIcon class="w-3.5 h-3.5 ml-1 shrink-0 text-stone-400" />
                    </button>

                    // Dropdown menu (opens upward)
                    <Show when=move || dropdown_open.get()>
                        <div class="absolute bottom-full left-0 right-0 mb-1 bg-stone-800 border border-stone-600 rounded shadow-lg z-50 max-h-60 overflow-y-auto">
                            // Recent caves
                            {move || {
                                let cfg = ctx.config.get();
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
                    <GearIcon />
                </button>
            </div>
        </div>
    }
}
