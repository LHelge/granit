use super::icons::Icon;
use crate::app::{ipc, AppCtx};
use leptos::prelude::*;

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
        <div class="border-t border-base-content/10 px-2 py-2">
            <div class="flex items-center gap-1">
                // Cave selector dropdown
                <div class="relative flex-1">
                    <button
                        class="w-full flex items-center justify-between px-2 py-1.5 text-sm bg-base-300 border border-base-content/20 rounded hover:border-base-content/30 transition-colors text-base-content/70 text-left truncate"
                        on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
                    >
                        <span class="truncate">{cave_label}</span>
                        <span class="inline-flex w-3.5 h-3.5 ml-1 shrink-0 text-base-content/50">
                            <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                        </span>
                    </button>

                    // Dropdown menu (opens upward)
                    <Show when=move || dropdown_open.get()>
                        <ul class="menu menu-sm absolute bottom-full left-0 right-0 mb-1 bg-base-300 border border-base-content/20 rounded shadow-lg z-50 max-h-60 overflow-y-auto py-1">
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
                                        <li>
                                            <button
                                                class="truncate"
                                                title=full_path
                                                on:click=move |_| open_and_refresh(path_clone.clone())
                                            >
                                                {display}
                                            </button>
                                        </li>
                                    }
                                }).collect_view()
                            }}

                            // Divider
                            <li><hr /></li>

                            // Single "Open folder…" button (replaces duplicate open/create)
                            <li>
                                <button on:click=on_pick_folder>
                                    "Open folder…"
                                </button>
                            </li>
                        </ul>
                    </Show>
                </div>

                // Settings gear icon
                <div class="tooltip tooltip-top" data-tip="Settings">
                    <button
                        class="btn btn-ghost btn-xs btn-square"
                        on:click=move |_| set_settings_open.set(true)
                    >
                        <Icon icon=icondata_lu::LuSettings width="1rem" height="1rem"/>
                    </button>
                </div>
            </div>
        </div>
    }
}
