use leptos::prelude::*;

use crate::app::ipc;
use crate::app::types::{Note, NoteMeta};

#[component]
pub fn NoteList(
    notes: ReadSignal<Vec<NoteMeta>>,
    set_active_note: WriteSignal<Option<Note>>,
) -> impl IntoView {
    let on_select = move |slug: String| {
        leptos::task::spawn_local(async move {
            if let Some(note) = ipc::read_note(&slug).await {
                set_active_note.set(Some(note));
            }
        });
    };

    move || {
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
