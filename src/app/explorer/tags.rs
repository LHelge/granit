use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::{resolve_note_icon, TagMap};
use leptos::prelude::*;

#[component]
pub fn Tags() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let tags = RwSignal::new(TagMap::default());
    let loading = RwSignal::new(false);

    let refresh = move || {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::list_tags().await {
                Ok(map) => tags.set(map),
                Err(e) => {
                    ctx.push_error("tags", format!("Failed to load tags: {e}"));
                }
            }
            loading.set(false);
        });
    };

    // Initial load when cave changes
    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            refresh();
        } else {
            tags.set(TagMap::default());
        }
    });

    // Re-fetch when cave contents change
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
                    && tags.get().tags.is_empty()
            }>
                <div class="flex-1 flex items-center justify-center p-4">
                    <p class="text-sm text-base-content/35 italic">"No tags found"</p>
                </div>
            </Show>

            // Tag accordion
            <Show when=move || !loading.get() && !tags.get().tags.is_empty()>
                <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10">
                    <div>
                        <p class="text-sm font-medium text-base-content/80">"Tags"</p>
                        <p class="text-xs text-base-content/40">"Notes grouped by tag"</p>
                    </div>
                    <span class="badge badge-accent badge-soft badge-sm">
                        {move || tags.get().tags.len()}
                    </span>
                </div>

                <div class="flex-1 overflow-y-auto">
                    {move || tags.get().tags.into_iter().map(|(tag, notes)| {
                        let tag_label = tag.clone();
                        let note_count = notes.len();
                        view! {
                            <div class="collapse collapse-arrow rounded-none border-b border-base-content/5">
                                <input type="radio" name="tags-accordion" />
                                <div class="collapse-title flex items-center gap-2 px-2 py-1.5 min-h-0 text-sm font-medium after:!top-5">
                                    <span class="badge badge-ghost badge-xs">{note_count}</span>
                                    <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                        <Icon icon=icondata_lu::LuTag width="100%" height="100%"/>
                                    </span>
                                    <span class="flex-1 min-w-0 truncate">{tag_label}</span>
                                </div>
                                <div class="collapse-content px-0 pb-0">
                                    <ul class="menu w-full menu-sm p-0">
                                        {notes.into_iter().map(|note| {
                                            let slug = note.slug.clone();
                                            let slug_open = slug.clone();
                                            let slug_label = slug.clone();
                                            let icon_id = note.icon.clone().unwrap_or_default();
                                            let is_active = move || {
                                                ctx.active_note
                                                    .get()
                                                    .map(|active| active.meta.slug == slug)
                                                    .unwrap_or(false)
                                            };

                                            view! {
                                                <li>
                                                    <button
                                                        class=move || {
                                                            if is_active() {
                                                                "flex w-full items-center gap-2 rounded-none bg-base-content/10 text-base-content"
                                                            } else {
                                                                "flex w-full items-center gap-2 rounded-none text-base-content/70 hover:bg-base-content/5 hover:text-base-content"
                                                            }
                                                        }
                                                        on:click=move |_| {
                                                            let current_slug = slug_open.clone();
                                                            leptos::task::spawn_local(async move {
                                                                match ipc::read_note(&current_slug).await {
                                                                    Ok(note) => ctx.set_active_note_document(note),
                                                                    Err(e) => {
                                                                        ctx.push_error("tags", format!("Failed to open note: {e}"));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    >
                                                        <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                            <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                                        </span>
                                                        <span class="flex-1 min-w-0 truncate text-left">
                                                            {slug_label}
                                                        </span>
                                                    </button>
                                                </li>
                                            }
                                        }).collect_view()}
                                    </ul>
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}
