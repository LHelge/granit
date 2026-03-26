use leptos::prelude::*;
use serde::Serialize;

use super::invoke;
use super::types::{Note, NoteMeta};

#[derive(Serialize)]
struct SaveNoteArgs {
    name: String,
    content: String,
}

/// Center panel — markdown editor with edit/read mode toggle.
#[component]
pub fn Editor(
    active_note: ReadSignal<Option<Note>>,
    set_active_note: WriteSignal<Option<Note>>,
    set_notes: WriteSignal<Vec<NoteMeta>>,
) -> impl IntoView {
    let (editing, set_editing) = signal(true);
    let (content, set_content) = signal(String::new());
    let (saving, set_saving) = signal(false);

    // When active_note changes, update the local content
    Effect::new(move || {
        if let Some(note) = active_note.get() {
            set_content.set(note.content.clone());
        } else {
            set_content.set(String::new());
        }
    });

    let toggle_mode = move |_| set_editing.update(|v| *v = !*v);

    let on_save = move |_| {
        let note = active_note.get_untracked();
        let content = content.get_untracked();
        if let Some(note) = note {
            set_saving.set(true);
            leptos::task::spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&SaveNoteArgs {
                    name: note.meta.slug.clone(),
                    content: content.clone(),
                })
                .unwrap();
                let result = invoke("save_note", args).await;
                if let Ok(meta) = serde_wasm_bindgen::from_value::<NoteMeta>(result) {
                    // Update active note with new content and meta
                    set_active_note.set(Some(Note { meta, content }));
                    // Refresh note list to pick up title changes
                    set_notes.set(super::fetch_notes().await);
                }
                set_saving.set(false);
            });
        }
    };

    let title = move || {
        active_note
            .get()
            .map(|n| n.meta.title.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    };

    let has_note = move || active_note.get().is_some();

    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-stone-900">
            // Toolbar
            <div class="flex items-center gap-2 px-3 py-1.5 border-b border-stone-700 shrink-0">
                <span class="text-sm text-stone-400 flex-1 truncate">{title}</span>
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
