use crate::app::components::icons::Icon;
use granit_types::{resolve_note_icon, NOTE_ICONS};
use leptos::prelude::*;

/// Searchable icon picker for the frontmatter editor.
///
/// Renders a trigger button showing the current icon (or a default), a clear
/// button, and a backdrop-assisted dropdown with a search field and 5-column
/// icon grid.
#[component]
pub(super) fn IconPicker(
    /// Currently selected icon ID (PascalCase, e.g. `"Star"`), or `None`.
    #[prop(into)]
    value: Signal<Option<String>>,
    /// Called when the user selects or clears an icon.
    #[prop(into)]
    on_change: Callback<Option<String>>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (search, set_search) = signal(String::new());

    let toggle = move |_: leptos::ev::MouseEvent| {
        let will_open = !open.get_untracked();
        set_open.set(will_open);
        if will_open {
            set_search.set(String::new());
        }
    };

    let close = move |_: leptos::ev::MouseEvent| {
        set_open.set(false);
    };

    let filtered_icons = move || {
        let q = search.get().to_lowercase();
        NOTE_ICONS
            .iter()
            .filter(move |e| {
                q.is_empty()
                    || e.id.to_lowercase().contains(&q)
                    || e.label.to_lowercase().contains(&q)
                    || e.tags.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };

    let trigger_icon =
        Signal::derive(move || resolve_note_icon(value.get().as_deref().unwrap_or("")));

    view! {
        <div class="relative">
            // Trigger: icon only, sized to match the reader's icon span
            <button
                type="button"
                class="inline-flex w-6 h-6 shrink-0 text-base-content/50 hover:text-base-content transition-colors"
                title="Change icon"
                on:click=toggle
            >
                <Icon icon=trigger_icon width="100%" height="100%"/>
            </button>

            // Dropdown
            <Show when=move || open.get()>
                // Invisible backdrop — closes picker on outside click
                <div class="fixed inset-0 z-40" on:click=close/>

                <div class="absolute left-0 top-7 z-50 w-72 bg-base-300 border border-base-content/20 rounded shadow-lg flex flex-col">
                    // Search input
                    <div class="p-2 border-b border-base-content/10">
                        <input
                            type="text"
                            class="w-full bg-base-100 border border-base-content/20 rounded px-2 py-1 text-xs text-base-content placeholder:text-base-content/35 outline-none focus:border-primary transition-colors"
                            placeholder="Search icons…"
                            prop:value=move || search.get()
                            on:input=move |ev| set_search.set(event_target_value(&ev))
                        />
                    </div>

                    // Icon grid (5 columns)
                    <div class="p-2 grid grid-cols-5 gap-1 max-h-48 overflow-y-auto">
                        {move || {
                            filtered_icons()
                                .into_iter()
                                .map(|entry| {
                                    let id = entry.id;
                                    let label = entry.label;
                                    let icon_data = entry.icon;
                                    let is_selected =
                                        move || value.get().as_deref() == Some(id);
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                let base = "flex items-center justify-center p-2 rounded text-base-content/50 hover:text-base-content hover:bg-base-content/10 transition-colors";
                                                if is_selected() {
                                                    format!("{base} bg-base-content/20 text-base-content")
                                                } else {
                                                    base.to_string()
                                                }
                                            }
                                            title=label
                                            on:click=move |_| {
                                                on_change.run(Some(id.to_string()));
                                                set_open.set(false);
                                            }
                                        >
                                            <span class="inline-flex w-4 h-4">
                                                <Icon icon=icon_data width="100%" height="100%"/>
                                            </span>
                                        </button>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
