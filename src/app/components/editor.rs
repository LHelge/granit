use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::ipc;
use granit_types::{AppConfig, Note, NoteMeta, RenderedNote};

#[component]
pub fn Editor(
    active_note: RwSignal<Option<Note>>,
    notes: RwSignal<Vec<NoteMeta>>,
    config: RwSignal<AppConfig>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let (content, set_content) = signal(String::new());
    let (title_input, set_title_input) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (prev_slug, set_prev_slug) = signal(None::<String>);
    let (rendered_note, set_rendered_note) = signal(None::<RenderedNote>);
    // When true, the next note switch opens in edit mode instead of preview.
    let (open_in_edit, set_open_in_edit) = signal(false);

    // When active_note changes, detect real note switches:
    // - auto-save the previous note if we were editing
    // - switch to preview mode and render the new note
    Effect::new(move || {
        let new_note = active_note.get();
        let old_slug = prev_slug.get_untracked();
        let was_editing = editing.get_untracked();
        let is_saving = saving.get_untracked();

        let new_slug = new_note.as_ref().map(|n| n.meta.slug.clone());
        let is_switch = old_slug != new_slug && !is_saving;

        if is_switch {
            // Auto-save the previous note when switching away in edit mode
            if was_editing {
                if let Some(slug) = old_slug {
                    let old_content = content.get_untracked();
                    let old_title = title_input.get_untracked().trim().to_string();
                    let new_name = if !old_title.is_empty() {
                        old_title
                    } else {
                        slug.clone()
                    };
                    leptos::task::spawn_local(async move {
                        if let Err(e) = ipc::update_note(&slug, &new_name, &old_content).await {
                            set_error.set(Some(format!("Autosave failed: {e}")));
                        }
                        if let Ok(n) = ipc::fetch_notes().await {
                            notes.set(n);
                        }
                    });
                }
            }
            // Open new note in preview or edit mode depending on flag
            let edit_next = open_in_edit.get_untracked();
            set_open_in_edit.set(false);
            set_editing.set(edit_next);
            match &new_note {
                Some(note) => {
                    let slug = note.meta.slug.clone();
                    leptos::task::spawn_local(async move {
                        match ipc::render_note(&slug).await {
                            Ok(rendered) => set_rendered_note.set(Some(rendered)),
                            Err(_) => set_rendered_note.set(None),
                        }
                    });
                }
                None => set_rendered_note.set(None),
            }
        }

        // Update local state
        if let Some(note) = new_note {
            set_prev_slug.set(Some(note.meta.slug.clone()));
            set_content.set(note.content.clone());
            set_title_input.set(note.meta.slug.clone());
        } else {
            set_prev_slug.set(None);
            set_content.set(String::new());
            set_title_input.set(String::new());
        }
        set_error.set(None);
    });

    // Dirty state: true when content or filename differs from the active note on disk
    let _is_dirty = Memo::new(move |_| match active_note.get() {
        Some(note) => content.get() != note.content || title_input.get() != note.meta.slug,
        None => false,
    });

    let toggle_mode = move |_| {
        let currently_editing = editing.get_untracked();
        set_editing.update(|v| *v = !*v);
        // Re-render when switching back to preview (content may have been edited)
        if currently_editing {
            if let Some(note) = active_note.get_untracked() {
                let slug = note.meta.slug.clone();
                leptos::task::spawn_local(async move {
                    if let Ok(rendered) = ipc::render_note(&slug).await {
                        set_rendered_note.set(Some(rendered));
                    }
                });
            }
        }
    };

    let on_save = move |_| {
        let note = active_note.get_untracked();
        let cur_content = content.get_untracked();
        let new_name = title_input.get_untracked().trim().to_string();
        if let Some(note) = note {
            if new_name.is_empty() {
                set_error.set(Some("Filename cannot be empty".to_string()));
                return;
            }

            set_saving.set(true);
            set_error.set(None);
            let old_slug = note.meta.slug.clone();

            leptos::task::spawn_local(async move {
                match ipc::update_note(&old_slug, &new_name, &cur_content).await {
                    Ok(meta) => {
                        // Update prev_slug immediately so the Effect does not
                        // mistake this save as a note switch when active_note changes.
                        set_prev_slug.set(Some(meta.slug.clone()));
                        let slug = meta.slug.clone();
                        active_note.set(Some(Note {
                            meta,
                            content: cur_content,
                        }));
                        if let Ok(n) = ipc::fetch_notes().await {
                            notes.set(n);
                        }
                        // Switch to preview and re-render with the saved content
                        set_editing.set(false);
                        if let Ok(rendered) = ipc::render_note(&slug).await {
                            set_rendered_note.set(Some(rendered));
                        }
                    }
                    Err(e) => {
                        set_error.set(Some(e));
                    }
                }
                set_saving.set(false);
            });
        }
    };

    // Intercept clicks on rendered wiki-links and navigate to the target note.
    // External links (http/https) and anchors (#) pass through normally.
    let on_prose_click = move |ev: leptos::ev::MouseEvent| {
        let Some(target) = ev.target() else { return };
        let anchor = target
            .dyn_ref::<web_sys::Element>()
            .and_then(|el| {
                if el.tag_name().eq_ignore_ascii_case("a") {
                    Some(el.clone())
                } else {
                    el.closest("a").ok().flatten()
                }
            })
            .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok());

        let Some(anchor) = anchor else { return };
        let href = anchor.get_attribute("href").unwrap_or_default();

        if href.is_empty()
            || href.starts_with("http")
            || href.starts_with('#')
            || href.starts_with('/')
        {
            return;
        }
        ev.prevent_default();
        // Decode percent-encoded characters (e.g. %20 → space) before lookup
        let slug = js_sys::decode_uri_component(&href)
            .ok()
            .and_then(|s| s.as_string())
            .unwrap_or(href);
        let is_broken = anchor.class_list().contains("broken-link");
        leptos::task::spawn_local(async move {
            if is_broken {
                // Create the note, open it in edit mode
                if let Ok(meta) = ipc::create_note(&slug).await {
                    if let Ok(note) = ipc::read_note(&meta.slug).await {
                        set_open_in_edit.set(true);
                        active_note.set(Some(note));
                        if let Ok(all) = ipc::fetch_notes().await {
                            notes.set(all);
                        }
                    }
                }
            } else if let Ok(note) = ipc::read_note(&slug).await {
                active_note.set(Some(note));
            }
        });
    };

    let has_note = move || active_note.get().is_some();

    let editor_style = move || {
        let cfg = config.get();
        format!(
            "font-family: {}; font-size: {}px;",
            cfg.editor.font_family, cfg.editor.font_size
        )
    };

    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-stone-900 relative">
            // Floating action buttons — always top-right, no layout impact
            <Show when=has_note>
                <div class="absolute top-3 right-4 z-10 flex items-center gap-1">
                    <Show
                        when=move || editing.get()
                        fallback=move || view! {
                            // Preview mode: pencil icon → switch to edit
                            <button
                                class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors"
                                title="Edit"
                                on:click=toggle_mode
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                                    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                                </svg>
                            </button>
                        }
                    >
                        // Edit mode: floppy disk → save, X → cancel
                        <button
                            class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors disabled:opacity-30"
                            title="Save"
                            on:click=on_save
                            disabled=move || saving.get()
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
                                <polyline points="17 21 17 13 7 13 7 21"/>
                                <polyline points="7 3 7 8 15 8"/>
                            </svg>
                        </button>
                        <button
                            class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors"
                            title="Cancel editing"
                            on:click=toggle_mode
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <line x1="18" y1="6" x2="6" y2="18"/>
                                <line x1="6" y1="6" x2="18" y2="18"/>
                            </svg>
                        </button>
                    </Show>
                </div>
            </Show>

            // Error banner
            <Show when=move || error.get().is_some()>
                <div class="px-4 py-1.5 bg-red-900/50 border-b border-red-700 text-red-300 text-xs flex items-center gap-2 shrink-0">
                    <span class="flex-1">{move || error.get().unwrap_or_default()}</span>
                    <button
                        class="text-red-400 hover:text-red-200"
                        on:click=move |_| set_error.set(None)
                    >
                        "✕"
                    </button>
                </div>
            </Show>

            // Content area — same padding and layout for both modes
            <div class="flex-1 overflow-y-auto px-8 pt-8 pb-12">
                <Show
                    when=has_note
                    fallback=|| view! {
                        <p class="text-stone-500 italic">"Select or create a note to get started"</p>
                    }
                >
                    <div class="prose prose-invert max-w-none">
                        <Show
                            when=move || editing.get()
                            fallback=move || view! {
                                // Preview: h1 title, date metadata, rendered body
                                <h1 class="!mt-0 !mb-1">
                                    {move || rendered_note.get().map(|r| r.title).unwrap_or_default()}
                                </h1>
                                {
                                    move || {
                                        let note = rendered_note.get()?;
                                        let items: Vec<(&str, String)> = [
                                            note.created_display.as_deref().map(|s| ("Created", s.to_string())),
                                            note.modified_display.as_deref().map(|s| ("Edited", s.to_string())),
                                        ]
                                        .into_iter()
                                        .flatten()
                                        .collect();
                                        if items.is_empty() {
                                            return None;
                                        }
                                        Some(view! {
                                            <ul class="not-prose list-none p-0 !mt-0 !mb-6">
                                                {items.into_iter().map(|(label, ts)| view! {
                                                    <li class="text-sm italic text-stone-500">
                                                        {format!("{label}: {ts}")}
                                                    </li>
                                                }).collect_view()}
                                            </ul>
                                        })
                                    }
                                }
                                <div
                                    style=editor_style
                                    inner_html=move || rendered_note.get().map(|r| r.html).unwrap_or_default()
                                    on:click=on_prose_click
                                />
                            }
                        >
                            // Edit: h1-styled title input, then markdown textarea
                            <input
                                type="text"
                                class="not-prose w-full bg-transparent text-white text-4xl font-extrabold leading-tight outline-none mb-2"
                                placeholder="Untitled"
                                prop:value=move || title_input.get()
                                on:input=move |ev| set_title_input.set(event_target_value(&ev))
                            />
                            <textarea
                                class="not-prose w-full min-h-[60vh] bg-transparent text-stone-300 resize-none outline-none leading-relaxed"
                                style=editor_style
                                placeholder="Start writing..."
                                prop:value=move || content.get()
                                on:input=move |ev| set_content.set(event_target_value(&ev))
                            />
                        </Show>
                    </div>
                </Show>
            </div>
        </main>
    }
}
