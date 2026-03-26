use leptos::prelude::*;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::invoke;
use super::types::{AppConfig, Note, NoteMeta};

#[derive(Serialize)]
struct OpenDialogOptions {
    directory: bool,
    multiple: bool,
}

/// Open the native folder picker and return the selected path (or None if cancelled).
async fn pick_folder() -> Option<String> {
    let tauri =
        js_sys::Reflect::get(&web_sys::window()?.into(), &JsValue::from_str("__TAURI__")).ok()?;
    let dialog = js_sys::Reflect::get(&tauri, &JsValue::from_str("dialog")).ok()?;
    let open_fn = js_sys::Reflect::get(&dialog, &JsValue::from_str("open")).ok()?;
    let open_fn = js_sys::Function::from(open_fn);

    let opts = serde_wasm_bindgen::to_value(&OpenDialogOptions {
        directory: true,
        multiple: false,
    })
    .ok()?;

    let promise = open_fn.call1(&JsValue::NULL, &opts).ok()?;
    let result: JsValue = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;
    result.as_string()
}

#[derive(Serialize)]
struct OpenCaveArgs {
    path: String,
}

/// Call the backend `open_cave` command and return the updated config.
async fn open_cave_cmd(path: &str) -> Option<AppConfig> {
    let args = serde_wasm_bindgen::to_value(&OpenCaveArgs {
        path: path.to_string(),
    })
    .ok()?;
    let result = invoke("open_cave", args).await.ok()?;
    serde_wasm_bindgen::from_value(result).ok()
}

#[derive(Serialize)]
struct CreateNoteArgs {
    name: String,
}

#[derive(Serialize)]
struct ReadNoteArgs {
    name: String,
}

/// Left sidebar — file tree for navigating the cave.
#[component]
pub fn Sidebar(
    config: ReadSignal<AppConfig>,
    set_config: WriteSignal<AppConfig>,
    set_settings_open: WriteSignal<bool>,
    notes: ReadSignal<Vec<NoteMeta>>,
    set_notes: WriteSignal<Vec<NoteMeta>>,
    set_active_note: WriteSignal<Option<Note>>,
) -> impl IntoView {
    let (dropdown_open, set_dropdown_open) = signal(false);

    let refresh_and_open_cave = move |set_config: WriteSignal<AppConfig>,
                                      set_notes: WriteSignal<Vec<NoteMeta>>,
                                      set_active_note: WriteSignal<Option<Note>>,
                                      set_dropdown_open: WriteSignal<bool>,
                                      path: String| {
        leptos::task::spawn_local(async move {
            if let Some(new_config) = open_cave_cmd(&path).await {
                set_config.set(new_config);
                let notes = super::fetch_notes().await;
                set_notes.set(notes);
                set_active_note.set(None);
            }
            set_dropdown_open.set(false);
        });
    };

    let open_existing = move |_| {
        let set_config = set_config;
        let set_notes = set_notes;
        let set_active_note = set_active_note;
        let set_dropdown_open = set_dropdown_open;
        leptos::task::spawn_local(async move {
            if let Some(path) = pick_folder().await {
                refresh_and_open_cave(
                    set_config,
                    set_notes,
                    set_active_note,
                    set_dropdown_open,
                    path,
                );
            } else {
                set_dropdown_open.set(false);
            }
        });
    };

    let create_new = move |_| {
        let set_config = set_config;
        let set_notes = set_notes;
        let set_active_note = set_active_note;
        let set_dropdown_open = set_dropdown_open;
        leptos::task::spawn_local(async move {
            if let Some(path) = pick_folder().await {
                refresh_and_open_cave(
                    set_config,
                    set_notes,
                    set_active_note,
                    set_dropdown_open,
                    path,
                );
            } else {
                set_dropdown_open.set(false);
            }
        });
    };

    let select_recent = move |path: String| {
        refresh_and_open_cave(
            set_config,
            set_notes,
            set_active_note,
            set_dropdown_open,
            path,
        );
    };

    let on_new_note = move |_| {
        let set_notes = set_notes;
        let set_active_note = set_active_note;
        leptos::task::spawn_local(async move {
            // Create an untitled note — backend handles name uniqueness
            let args = serde_wasm_bindgen::to_value(&CreateNoteArgs {
                name: "untitled".to_string(),
            })
            .unwrap();
            let Ok(result) = invoke("create_note", args).await else {
                return;
            };
            if let Ok(meta) = serde_wasm_bindgen::from_value::<NoteMeta>(result) {
                let slug = meta.slug.clone();
                // Refresh note list
                set_notes.set(super::fetch_notes().await);
                // Open the new note
                let read_args = serde_wasm_bindgen::to_value(&ReadNoteArgs { name: slug }).unwrap();
                let Ok(note_result) = invoke("read_note", read_args).await else {
                    return;
                };
                if let Ok(note) = serde_wasm_bindgen::from_value::<Note>(note_result) {
                    set_active_note.set(Some(note));
                }
            }
        });
    };

    let on_select_note = move |slug: String| {
        let set_active_note = set_active_note;
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&ReadNoteArgs { name: slug }).unwrap();
            let Ok(result) = invoke("read_note", args).await else {
                return;
            };
            if let Ok(note) = serde_wasm_bindgen::from_value::<Note>(result) {
                set_active_note.set(Some(note));
            }
        });
    };

    let cave_label = move || {
        let cfg = config.get();
        cfg.recent_caves
            .first()
            .and_then(|p| p.rsplit('/').next().or_else(|| p.rsplit('\\').next()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "No cave open".to_string())
    };

    let has_cave = move || !config.get().recent_caves.is_empty();

    view! {
        <aside class="w-64 shrink-0 bg-stone-850 border-r border-stone-700 flex flex-col overflow-hidden">
            // Header
            <div class="flex items-center justify-between px-3 py-2 border-b border-stone-700">
                <span class="text-xs font-semibold uppercase tracking-wider text-stone-400">"Explorer"</span>
                <Show when=has_cave>
                    <button
                        class="p-0.5 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
                        title="New note"
                        on:click=on_new_note
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                        </svg>
                    </button>
                </Show>
            </div>

            // Note list
            <div class="flex-1 overflow-y-auto">
                <Show
                    when=has_cave
                    fallback=|| view! { <p class="p-2 text-sm text-stone-500 italic">"No cave open"</p> }
                >
                    {move || {
                        let note_list = notes.get();
                        if note_list.is_empty() {
                            view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }.into_any()
                        } else {
                            note_list.into_iter().map(|meta| {
                                let slug = meta.slug.clone();
                                let display = meta.slug;
                                view! {
                                    <button
                                        class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700/50 transition-colors truncate"
                                        on:click=move |_| on_select_note(slug.clone())
                                    >
                                        {display}
                                    </button>
                                }
                            }).collect_view().into_any()
                        }
                    }}
                </Show>
            </div>

            // Bottom bar: cave selector + settings gear
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
                                                on:click=move |_| select_recent(path_clone.clone())
                                            >
                                                {display}
                                            </button>
                                        }
                                    }).collect_view()
                                }}

                                // Divider
                                <div class="border-t border-stone-600 my-1"></div>

                                // Open existing
                                <button
                                    class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700 transition-colors"
                                    on:click=open_existing
                                >
                                    "Open existing…"
                                </button>

                                // Create new
                                <button
                                    class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700 transition-colors"
                                    on:click=create_new
                                >
                                    "Create new…"
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
        </aside>
    }
}
