use leptos::prelude::*;

use crate::app::ipc;
use crate::app::types::{AppConfig, Note, NoteMeta};

use super::cave_selector::CaveSelector;
use super::note_list::NoteList;

#[component]
pub fn Sidebar(
    config: ReadSignal<AppConfig>,
    set_config: WriteSignal<AppConfig>,
    set_settings_open: WriteSignal<bool>,
    notes: ReadSignal<Vec<NoteMeta>>,
    set_notes: WriteSignal<Vec<NoteMeta>>,
    set_active_note: WriteSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let on_new_note = move |_| {
        leptos::task::spawn_local(async move {
            let meta = match ipc::create_note("untitled").await {
                Ok(m) => m,
                Err(e) => {
                    error_msg.set(Some(format!("Failed to create note: {e}")));
                    return;
                }
            };
            let slug = meta.slug.clone();
            match ipc::fetch_notes().await {
                Ok(n) => {
                    notes_error.set(None);
                    set_notes.set(n);
                }
                Err(e) => notes_error.set(Some(e)),
            }
            match ipc::read_note(&slug).await {
                Ok(note) => set_active_note.set(Some(note)),
                Err(e) => error_msg.set(Some(format!("Failed to open note: {e}"))),
            }
        });
    };

    let has_cave = move || config.get().active_cave.is_some();

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
                    <NoteList
                        notes=notes
                        set_active_note=set_active_note
                        error_msg=error_msg
                        notes_error=notes_error
                    />
                </Show>
            </div>

            // Bottom bar: cave selector + settings
            <CaveSelector
                config=config
                set_config=set_config
                set_notes=set_notes
                set_active_note=set_active_note
                set_settings_open=set_settings_open
                error_msg=error_msg
                notes_error=notes_error
            />
        </aside>
    }
}
