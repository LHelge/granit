use leptos::prelude::*;

use crate::app::ipc;
use granit_types::{Note, NoteMeta};

#[component]
pub fn NoteList(
    notes: RwSignal<Vec<NoteMeta>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let on_select = move |slug: String| {
        leptos::task::spawn_local(async move {
            match ipc::read_note(&slug).await {
                Ok(note) => active_note.set(Some(note)),
                Err(e) => error_msg.set(Some(format!("Failed to load note: {e}"))),
            }
        });
    };

    move || {
        if let Some(err) = notes_error.get() {
            return view! {
                <p class="p-2 text-sm text-red-400 italic">{format!("Error loading notes: {err}")}</p>
            }
            .into_any();
        }
        let note_list = notes.get();
        if note_list.is_empty() {
            view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }.into_any()
        } else {
            note_list
                .into_iter()
                .map(|meta| {
                    let slug = meta.slug.clone();
                    let display = meta.slug;
                    view! {
                        <button
                            class="w-full text-left px-3 py-1.5 text-sm text-stone-300 hover:bg-stone-700/50 transition-colors truncate"
                            on:click=move |_| on_select(slug.clone())
                        >
                            {display}
                        </button>
                    }
                })
                .collect_view()
                .into_any()
        }
    }
}
