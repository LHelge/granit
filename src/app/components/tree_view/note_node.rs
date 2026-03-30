use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use granit_types::{Note, NoteMeta};

use super::rename_input::RenameInput;
use super::{refresh_tree, ContextMenu, ContextTarget, DragPayload, RenameTarget};
use crate::app::ipc;

/// Renders a single note row in the tree, with drag, context menu, and rename support.
#[component]
pub(super) fn NoteNode(
    meta: NoteMeta,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    context_menu: RwSignal<Option<ContextMenu>>,
    drag_payload: RwSignal<Option<DragPayload>>,
    renaming: RwSignal<Option<RenameTarget>>,
    indent_style: String,
) -> impl IntoView {
    let slug = meta.slug.clone();
    let slug_drag = meta.slug.clone();
    let slug_rename = meta.slug.clone();
    let slug_rename_check = meta.slug.clone();
    let display = meta.slug.clone();

    let is_renaming = move || matches!(renaming.get(), Some(RenameTarget::Note(ref s)) if *s == slug_rename_check);
    let on_dragstart = move |e: DragEvent| {
        e.stop_propagation();
        if let Some(dt) = e.data_transfer() {
            let _ = dt.set_data("text/plain", &slug_drag);
        }
        drag_payload.set(Some(DragPayload::Note(slug_drag.clone())));
    };

    view! {
        <li draggable="true" on:dragstart=on_dragstart>
            {move || {
                if is_renaming() {
                    let slug_for_rename = slug_rename.clone();
                    view! {
                        <RenameInput
                            initial=slug_for_rename.clone()
                            indent_style=indent_style.clone()
                            icon="note"
                            on_confirm=Callback::new(move |new_name: String| {
                                let old = slug_for_rename.clone();
                                renaming.set(None);
                                if new_name == old || new_name.is_empty() {
                                    return;
                                }
                                leptos::task::spawn_local(async move {
                                    match ipc::rename_note(&old, &new_name).await {
                                        Ok(new_meta) => {
                                            if active_note.get().map(|n| n.meta.slug == old).unwrap_or(false) {
                                                if let Ok(note) = ipc::read_note(&new_meta.slug).await {
                                                    active_note.set(Some(note));
                                                }
                                            }
                                            refresh_tree(notes, folders, error_msg).await;
                                        }
                                        Err(e) => error_msg.set(Some(format!("Failed to rename note: {e}"))),
                                    }
                                });
                            })
                            on_cancel=Callback::new(move |()| renaming.set(None))
                        />
                    }
                    .into_any()
                } else {
                    let slug_click = display.clone();
                    let slug_ctx = display.clone();
                    let slug_active = slug.clone();
                    view! {
                        <button
                            class=move || {
                                let base = "w-full text-left py-1 text-sm truncate transition-colors flex items-center gap-1";
                                if active_note.get().map(|n| n.meta.slug == slug_active).unwrap_or(false) {
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
                                        Ok(note) => active_note.set(Some(note)),
                                        Err(e) => error_msg.set(Some(format!("Failed to load note: {e}"))),
                                    }
                                });
                            }
                            on:contextmenu=move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();
                                context_menu.set(Some(ContextMenu {
                                    x: e.client_x(),
                                    y: e.client_y(),
                                    target: ContextTarget::Note(slug_ctx.clone()),
                                }));
                            }
                        >
                            <span class="w-3 shrink-0" />
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0 text-stone-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                            </svg>
                            {display.clone()}
                        </button>
                    }
                    .into_any()
                }
            }}
        </li>
    }
}
