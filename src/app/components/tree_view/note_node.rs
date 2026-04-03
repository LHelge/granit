use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use granit_types::NoteMeta;

use super::rename_input::RenameInput;
use super::{use_tree_ctx, ContextMenu, ContextTarget, DragPayload, RenameTarget};
use crate::app::components::icons::Icon;
use crate::app::ipc;
use granit_types::resolve_note_icon;

/// Renders a single note row in the tree, with drag, context menu, and rename support.
#[component]
pub(super) fn NoteNode(meta: NoteMeta, indent_style: String) -> impl IntoView {
    let ctx = use_tree_ctx();
    let slug = meta.slug.clone();

    let on_dragstart = {
        let slug = slug.clone();
        move |e: DragEvent| {
            e.stop_propagation();
            if let Some(dt) = e.data_transfer() {
                let _ = dt.set_data("text/plain", &slug);
            }
            ctx.drag_payload.set(Some(DragPayload::Note(slug.clone())));
        }
    };

    view! {
        <li draggable="true" on:dragstart=on_dragstart>
            {move || {
                let is_renaming = matches!(
                    ctx.renaming.get(),
                    Some(RenameTarget::Note(ref s)) if *s == slug
                );

                if is_renaming {
                    let slug = slug.clone();
                    view! {
                        <RenameInput
                            initial=slug.clone()
                            indent_style=indent_style.clone()
                            note=true
                            on_confirm=Callback::new(move |new_name: String| {
                                let old = slug.clone();
                                ctx.renaming.set(None);
                                if new_name == old || new_name.is_empty() {
                                    return;
                                }
                                leptos::task::spawn_local(async move {
                                    match ipc::rename_note(&old, &new_name).await {
                                        Ok(new_meta) => {
                                            if ctx.active_note.get().map(|n| n.meta.slug == old).unwrap_or(false) {
                                                if let Ok(note) = ipc::read_note(&new_meta.slug).await {
                                                    ctx.active_note.set(Some(note));
                                                }
                                            }
                                            ctx.refresh_async().await;
                                        }
                                        Err(e) => ctx.push_error(format!("Failed to rename note: {e}")),
                                    }
                                });
                            })
                            on_cancel=Callback::new(move |()| ctx.renaming.set(None))
                        />
                    }
                    .into_any()
                } else {
                    let slug = slug.clone();
                    let slug_click = slug.clone();
                    let slug_ctx = slug.clone();
                    view! {
                        <button
                            class=move || {
                                let base = "w-full text-left py-1 text-sm truncate transition-colors flex items-center gap-1";
                                if ctx.active_note.get().map(|n| n.meta.slug == slug).unwrap_or(false) {
                                    format!("{base} bg-stone-700 text-stone-100")
                                } else {
                                    format!("{base} text-stone-300 hover:bg-stone-700/50")
                                }
                            }
                            style=indent_style.clone()
                            on:click=move |_| {
                                let s = slug_click.clone();
                                leptos::task::spawn_local(async move {
                                    match ipc::read_note(&s).await {
                                        Ok(note) => ctx.active_note.set(Some(note)),
                                        Err(e) => ctx.push_error(format!("Failed to load note: {e}")),
                                    }
                                });
                            }
                            on:contextmenu=move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();
                                ctx.context_menu.set(Some(ContextMenu {
                                    x: e.client_x(),
                                    y: e.client_y(),
                                    target: ContextTarget::Note(slug_ctx.clone()),
                                }));
                            }
                        >
                            <span class="w-3 shrink-0" />
                            <span class="inline-flex w-3.5 h-3.5 shrink-0 text-stone-500">
                                <Icon icon=resolve_note_icon(meta.icon.as_deref().unwrap_or("")) width="100%" height="100%"/>
                            </span>
                            {slug_click.clone()}
                        </button>
                    }
                    .into_any()
                }
            }}
        </li>
    }
}
