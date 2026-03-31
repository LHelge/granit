use super::cave_selector::CaveSelector;
use super::tree_view::TreeView;
use granit_types::{AppConfig, Note, NoteMeta};
use leptos::prelude::*;

#[component]
pub fn Sidebar(
    config: RwSignal<AppConfig>,
    set_settings_open: WriteSignal<bool>,
    notes: RwSignal<Vec<NoteMeta>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let has_cave = move || config.get().active_cave.is_some();

    view! {
        <aside class="w-64 shrink-0 bg-stone-850 border-r border-stone-700 flex flex-col overflow-hidden">
            // Note list
            <div class="flex-1 overflow-y-auto">
                <Show
                    when=has_cave
                    fallback=|| view! { <p class="p-2 text-sm text-stone-500 italic">"No cave open"</p> }
                >
                    <TreeView
                        notes=notes
                        active_note=active_note
                        error_msg=error_msg
                        notes_error=notes_error
                    />
                </Show>
            </div>

            // Bottom bar: cave selector + settings
            <CaveSelector
                config=config
                notes=notes
                active_note=active_note
                set_settings_open=set_settings_open
                error_msg=error_msg
                notes_error=notes_error
            />
        </aside>
    }
}
