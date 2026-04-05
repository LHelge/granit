use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::TodoItem;
use leptos::prelude::*;
use std::collections::BTreeMap;

/// Render a list of todos grouped by note slug.
///
/// `completed` controls the visual styling (strikethrough, checked checkbox).
/// The caller is responsible for passing either all-incomplete or all-completed items.
#[component]
pub fn TodoGroups(todos: Signal<Vec<TodoItem>>, completed: bool) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    view! {
        <div>
            {move || {
                // Group todos by slug, preserving order (BTreeMap sorts by slug)
                let mut groups: BTreeMap<String, Vec<TodoItem>> = BTreeMap::new();
                for todo in todos.get() {
                    groups.entry(todo.slug.clone()).or_default().push(todo);
                }

                groups
                    .into_iter()
                    .map(|(slug, items)| {
                        let slug_open = slug.clone();
                        let open_note = move |_| {
                            let s = slug_open.clone();
                            leptos::task::spawn_local(async move {
                                match ipc::read_note(&s).await {
                                    Ok(note) => ctx.active_note.set(Some(note)),
                                    Err(e) => {
                                        ctx.push_error("todo", format!("Failed to open note: {e}"));
                                    }
                                }
                            });
                        };

                        view! {
                            <div class="mb-1">
                                // Note group header
                                <button
                                    class="flex items-center gap-1 w-full px-2 py-0.5 text-xs font-semibold text-base-content/50 hover:text-base-content transition-colors text-left truncate"
                                    on:click=open_note.clone()
                                >
                                    <span class="inline-flex w-3 h-3 shrink-0 text-base-content/40">
                                        <Icon icon=icondata_lu::LuFileText width="100%" height="100%"/>
                                    </span>
                                    <span class="truncate">{slug.clone()}</span>
                                </button>

                                // Todo items in this note
                                <ul class="menu menu-sm p-0">
                                    {items.into_iter().map(|item| {
                                        let item_slug = item.slug.clone();
                                        let item_slug_open = item.slug.clone();
                                        let item_line = item.line;

                                        view! {
                                            <li>
                                                <div class="flex items-start gap-2 px-2 py-1 rounded-none hover:bg-base-content/5 cursor-default">
                                                    // Checkbox
                                                    <input
                                                        type="checkbox"
                                                        class="checkbox checkbox-xs shrink-0 mt-0.5"
                                                        prop:checked=completed
                                                        on:click=move |ev| {
                                                            ev.prevent_default();
                                                            let s = item_slug.clone();
                                                            let l = item_line;
                                                            leptos::task::spawn_local(async move {
                                                                if let Err(e) = ipc::toggle_todo(&s, l).await {
                                                                    ctx.push_error("todo", format!("Failed to toggle todo: {e}"));
                                                                }
                                                            });
                                                        }
                                                    />
                                                    // Text (clickable — opens note)
                                                    <button
                                                        class="flex-1 text-left text-sm break-words"
                                                        class:line-through=completed
                                                        class:opacity-40=completed
                                                        on:click=move |_| {
                                                            let s = item_slug_open.clone();
                                                            leptos::task::spawn_local(async move {
                                                                match ipc::read_note(&s).await {
                                                                    Ok(note) => ctx.active_note.set(Some(note)),
                                                                    Err(e) => {
                                                                        ctx.push_error(
                                                                            "todo",
                                                                            format!("Failed to open note: {e}"),
                                                                        );
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    >
                                                        {item.text}
                                                    </button>
                                                </div>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}
