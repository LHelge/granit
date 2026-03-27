use leptos::prelude::*;

use crate::app::ipc;
use crate::app::types::{Note, NoteMeta};

#[component]
pub fn Editor(
    active_note: ReadSignal<Option<Note>>,
    set_active_note: WriteSignal<Option<Note>>,
    set_notes: WriteSignal<Vec<NoteMeta>>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let (content, set_content) = signal(String::new());
    let (title_input, set_title_input) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (prev_slug, set_prev_slug) = signal(None::<String>);

    // When active_note changes, detect real note switches:
    // - auto-save the previous note if we were editing
    // - switch to preview mode
    Effect::new(move || {
        let new_note = active_note.get();
        let old_slug = prev_slug.get_untracked();
        let was_editing = editing.get_untracked();
        let is_saving = saving.get_untracked();

        let new_slug = new_note.as_ref().map(|n| n.meta.slug.clone());
        let is_switch = old_slug != new_slug && !is_saving;

        if is_switch {
            // Auto-save previous note content when switching away in edit mode
            if was_editing {
                if let Some(slug) = old_slug {
                    let old_content = content.get_untracked();
                    let old_title = title_input.get_untracked().trim().to_string();
                    let set_notes = set_notes;
                    leptos::task::spawn_local(async move {
                        // Rename if the title changed
                        let current_slug = if !old_title.is_empty() && old_title != slug {
                            match ipc::rename_note(&slug, &old_title).await {
                                Ok(meta) => meta.slug,
                                Err(_) => slug,
                            }
                        } else {
                            slug
                        };
                        // Save content
                        let _ = ipc::save_note(&current_slug, &old_content).await;
                        if let Ok(n) = ipc::fetch_notes().await {
                            set_notes.set(n);
                        }
                    });
                }
            }
            // Open new note in preview mode
            set_editing.set(false);
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

    let toggle_mode = move |_| set_editing.update(|v| *v = !*v);

    let on_save = move |_| {
        let note = active_note.get_untracked();
        let content = content.get_untracked();
        let new_title = title_input.get_untracked().trim().to_string();
        if let Some(note) = note {
            if new_title.is_empty() {
                set_error.set(Some("Title cannot be empty".to_string()));
                return;
            }

            set_saving.set(true);
            set_error.set(None);
            let old_slug = note.meta.slug.clone();

            leptos::task::spawn_local(async move {
                // If the title changed, rename the file first
                let current_slug = if new_title != old_slug {
                    match ipc::rename_note(&old_slug, &new_title).await {
                        Ok(meta) => meta.slug,
                        Err(e) => {
                            set_error.set(Some(e));
                            set_saving.set(false);
                            return;
                        }
                    }
                } else {
                    old_slug
                };

                // Save the content
                match ipc::save_note(&current_slug, &content).await {
                    Ok(meta) => {
                        set_active_note.set(Some(Note { meta, content }));
                        if let Ok(n) = ipc::fetch_notes().await {
                            set_notes.set(n);
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

    let has_note = move || active_note.get().is_some();

    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-stone-900">
            // Toolbar
            <div class="flex items-center gap-2 px-3 py-1.5 border-b border-stone-700 shrink-0">
                <Show
                    when=move || editing.get() && has_note()
                    fallback=move || view! {
                        <span class="text-sm text-stone-400 flex-1 truncate">
                            {move || title_input.get()}
                        </span>
                    }
                >
                    <input
                        type="text"
                        class="text-sm flex-1 bg-transparent text-stone-200 outline-none border-b border-stone-600 focus:border-stone-400 transition-colors"
                        placeholder="Note title…"
                        prop:value=move || title_input.get()
                        on:input=move |ev| set_title_input.set(event_target_value(&ev))
                    />
                </Show>
                <Show when=has_note>
                    <button
                        class="px-2 py-0.5 text-xs rounded border border-stone-600 text-stone-300 hover:bg-stone-700 transition-colors disabled:opacity-50"
                        on:click=on_save
                        disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Saving…" } else { "Save" }}
                    </button>
                    <button
                        class="px-2 py-0.5 text-xs rounded border border-stone-600 text-stone-300 hover:bg-stone-700 transition-colors"
                        on:click=toggle_mode
                    >
                        {move || if editing.get() { "Preview" } else { "Edit" }}
                    </button>
                </Show>
            </div>

            // Error banner
            <Show when=move || error.get().is_some()>
                <div class="px-3 py-1.5 bg-red-900/50 border-b border-red-700 text-red-300 text-xs flex items-center gap-2">
                    <span class="flex-1">{move || error.get().unwrap_or_default()}</span>
                    <button
                        class="text-red-400 hover:text-red-200"
                        on:click=move |_| set_error.set(None)
                    >
                        "✕"
                    </button>
                </div>
            </Show>

            // Content area
            <div class="flex-1 overflow-y-auto p-6">
                <Show
                    when=has_note
                    fallback=|| view! {
                        <p class="text-stone-500 italic">"Select or create a note to get started"</p>
                    }
                >
                    <Show
                        when=move || editing.get()
                        fallback=move || view! {
                            <div class="prose prose-invert max-w-none">
                                <p class="text-stone-300 whitespace-pre-wrap">{move || content.get()}</p>
                            </div>
                        }
                    >
                        <textarea
                            class="w-full h-full bg-transparent text-stone-200 resize-none outline-none font-mono text-sm leading-relaxed"
                            placeholder="Start writing..."
                            prop:value=move || content.get()
                            on:input=move |ev| set_content.set(event_target_value(&ev))
                        />
                    </Show>
                </Show>
            </div>
        </main>
    }
}
