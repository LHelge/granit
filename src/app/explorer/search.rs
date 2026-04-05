use crate::app::{ipc, AppCtx};
use granit_types::ContentMatch;
use leptos::prelude::*;

#[component]
pub fn Search() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let (query, set_query) = signal(String::new());
    let (results, set_results) = signal(Vec::<ContentMatch>::new());
    let (searching, set_searching) = signal(false);

    let do_search = move || {
        let q = query.get();
        if q.trim().is_empty() {
            set_results.set(Vec::new());
            return;
        }
        set_searching.set(true);
        leptos::task::spawn_local(async move {
            match ipc::search_content(&q).await {
                Ok(matches) => set_results.set(matches),
                Err(e) => {
                    ctx.push_error("search", format!("Search failed: {e}"));
                    set_results.set(Vec::new());
                }
            }
            set_searching.set(false);
        });
    };

    view! {
        <div class="flex flex-col h-full">
            // Search input
            <div class="p-2">
                <input
                    type="text"
                    placeholder="Search notes…"
                    class="input input-sm w-full"
                    prop:value=move || query.get()
                    on:input=move |ev| {
                        set_query.set(event_target_value(&ev));
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            do_search();
                        }
                    }
                />
            </div>

            // Results list
            <div class="flex-1 overflow-y-auto">
                <Show when=move || searching.get()>
                    <div class="flex justify-center p-4">
                        <span class="loading loading-spinner loading-sm"></span>
                    </div>
                </Show>
                <Show when=move || !searching.get() && !query.get().trim().is_empty() && results.get().is_empty()>
                    <p class="p-2 text-sm text-base-content/35 italic">"No results"</p>
                </Show>
                <ul class="menu menu-sm p-0">
                    {move || results.get().into_iter().map(|m| {
                        let slug = m.slug.clone();
                        let slug_click = slug.clone();
                        let first_snippet = m.snippets.first().cloned().unwrap_or_default();
                        let extra_count = m.snippets.len().saturating_sub(1);
                        view! {
                            <li>
                                <button
                                    class="flex flex-col items-start gap-0 rounded-none"
                                    on:click=move |_| {
                                        let s = slug_click.clone();
                                        leptos::task::spawn_local(async move {
                                            match ipc::read_note(&s).await {
                                                Ok(note) => ctx.active_note.set(Some(note)),
                                                Err(e) => { ctx.push_error("search", format!("Failed to open note: {e}")); }
                                            }
                                        });
                                    }
                                >
                                    <span class="font-medium text-base-content truncate w-full">{slug}</span>
                                    <span class="text-xs text-base-content/50 whitespace-normal break-words w-full">
                                        {first_snippet}
                                    </span>
                                    {if extra_count > 0 {
                                        Some(view! {
                                            <span class="text-xs text-base-content/30 italic">
                                                {format!("+ {extra_count} more match{}", if extra_count == 1 { "" } else { "es" })}
                                            </span>
                                        })
                                    } else {
                                        None
                                    }}
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </div>
        </div>
    }
}
