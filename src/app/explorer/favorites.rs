use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::resolve_note_icon;
use leptos::prelude::*;

#[component]
pub fn Favorites() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    let favorites = Signal::derive(move || {
        ctx.notes
            .get()
            .into_iter()
            .filter(|note| note.favorite.unwrap_or(false))
            .collect::<Vec<_>>()
    });

    view! {
        <div class="flex flex-col h-full">
            <Show
                when=move || ctx.config.get().active_cave.is_some()
                fallback=|| view! {
                    <div class="flex-1 flex items-center justify-center p-4">
                        <p class="text-sm text-base-content/35 italic">"No cave open"</p>
                    </div>
                }
            >
                <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10">
                    <div>
                        <p class="text-sm font-medium text-base-content/80">"Favorites"</p>
                        <p class="text-xs text-base-content/40">"Quick access to starred notes"</p>
                    </div>
                    <span class="badge badge-warning badge-soft badge-sm">
                        {move || favorites.get().len()}
                    </span>
                </div>

                <Show
                    when=move || !favorites.get().is_empty()
                    fallback=|| view! {
                        <div class="flex-1 flex items-center justify-center p-4">
                            <p class="text-sm text-base-content/35 italic">"No favorite notes yet"</p>
                        </div>
                    }
                >
                    <ul class="menu w-full menu-sm p-0 flex-1 overflow-y-auto">
                        {move || favorites.get().into_iter().map(|note| {
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
                                                "flex w-full items-center gap-2.5 rounded-none bg-base-content/10 text-base-content"
                                            } else {
                                                "flex w-full items-center gap-2.5 rounded-none text-base-content/70 hover:bg-base-content/5 hover:text-base-content"
                                            }
                                        }
                                        on:click=move |_| {
                                            let current_slug = slug_open.clone();
                                            leptos::task::spawn_local(async move {
                                                match ipc::read_note(&current_slug).await {
                                                    Ok(note) => ctx.set_active_note_document(note),
                                                    Err(e) => {
                                                        ctx.push_error("favorites", format!("Failed to open note: {e}"));
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        <span class="inline-flex w-4 h-4 shrink-0 text-accent">
                                            <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                        </span>
                                        <span class="inline-flex w-3.5 h-3.5 shrink-0 text-warning">
                                            <Icon icon=icondata_lu::LuStar width="100%" height="100%"/>
                                        </span>
                                        <span class="flex-1 min-w-0 truncate text-left text-[0.95rem] font-medium">
                                            {slug_label}
                                        </span>
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </Show>
            </Show>
        </div>
    }
}
