mod groups;

use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::TodoList;
use groups::TodoGroups;
use leptos::prelude::*;

#[component]
pub fn Todo() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let todos = RwSignal::new(TodoList::default());
    let show_completed = RwSignal::new(false);
    let loading = RwSignal::new(false);

    // Fetch todos from backend
    let refresh = move || {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::list_todos().await {
                Ok(list) => todos.set(list),
                Err(e) => {
                    ctx.push_error("todo", format!("Failed to load todos: {e}"));
                }
            }
            loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            refresh();
        } else {
            todos.set(TodoList::default());
        }
    });

    // Re-fetch when cave contents change (agent tools, file saves, etc.)
    Effect::new(move |_| {
        ipc::spawn_event_listener_simple("cave:notes-changed", move || {
            if ctx.config.get_untracked().active_cave.is_some() {
                refresh();
            }
        });
    });

    view! {
        <div class="flex flex-col h-full">
            // Loading indicator
            <Show when=move || loading.get()>
                <div class="flex justify-center p-4">
                    <span class="loading loading-spinner loading-sm"></span>
                </div>
            </Show>

            // No cave open
            <Show when=move || !loading.get() && ctx.config.get().active_cave.is_none()>
                <div class="flex-1 flex items-center justify-center p-4">
                    <p class="text-sm text-base-content/35 italic">"No cave open"</p>
                </div>
            </Show>

            // Empty state
            <Show when=move || {
                !loading.get()
                    && ctx.config.get().active_cave.is_some()
                    && todos.get().incomplete.is_empty()
                    && todos.get().completed.is_empty()
            }>
                <div class="flex-1 flex items-center justify-center p-4">
                    <p class="text-sm text-base-content/35 italic">"No todos found"</p>
                </div>
            </Show>

            // Todo list
            <Show when=move || {
                !loading.get()
                    && (!todos.get().incomplete.is_empty() || !todos.get().completed.is_empty())
            }>
                <div class="flex-1 overflow-y-auto">
                    // Incomplete todos, grouped by note
                    <TodoGroups
                        todos=Signal::derive(move || todos.get().incomplete)
                        completed=false
                    />

                    // Completed section (collapsible)
                    {move || {
                        let completed = todos.get().completed;
                        if completed.is_empty() {
                            return view! { <div></div> }.into_any();
                        }
                        let count = completed.len();
                        view! {
                            <div>
                                <button
                                    class="flex items-center gap-1 w-full px-2 py-1 text-xs font-semibold text-base-content/50 uppercase tracking-wide hover:text-base-content transition-colors"
                                    on:click=move |_| show_completed.update(|v| *v = !*v)
                                >
                                    <span class="inline-flex w-3 h-3 transition-transform" class:rotate-90=move || show_completed.get()>
                                        <Icon icon=icondata_lu::LuChevronRight width="100%" height="100%"/>
                                    </span>
                                    {format!("Completed ({count})")}
                                </button>
                                <Show when=move || show_completed.get()>
                                    <TodoGroups
                                        todos=Signal::derive(move || todos.get().completed)
                                        completed=true
                                    />
                                </Show>
                            </div>
                        }.into_any()
                    }}
                </div>
            </Show>
        </div>
    }
}
