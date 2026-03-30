use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use granit_types::{Note, NoteMeta};

use super::rename_input::RenameInput;
use super::tree_model::TreeNode;
use super::{handle_drop, refresh_tree, render_node};
use super::{ContextMenu, ContextTarget, DragPayload, RenameTarget};
use crate::app::ipc;

/// Renders a folder row in the tree, with collapsing, drag-drop, context menu,
/// and rename support. Recursively renders children.
#[allow(clippy::too_many_arguments)]
#[component]
pub(super) fn FolderNode(
    name: String,
    path: String,
    children: Vec<TreeNode>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    context_menu: RwSignal<Option<ContextMenu>>,
    drag_payload: RwSignal<Option<DragPayload>>,
    renaming: RwSignal<Option<RenameTarget>>,
    indent_style: String,
    depth: usize,
) -> impl IntoView {
    let open = RwSignal::new(true);
    let drag_over = RwSignal::new(false);

    let children_views = children
        .into_iter()
        .map(|child| {
            render_node(
                child,
                active_note,
                error_msg,
                notes,
                folders,
                context_menu,
                drag_payload,
                renaming,
                depth + 1,
            )
        })
        .collect_view();

    let path_drag = path.clone();
    let path_rename = path.clone();
    let path_rename_check = path.clone();
    let name_for_rename = name.clone();

    let on_dragstart = move |e: DragEvent| {
        e.stop_propagation();
        if let Some(dt) = e.data_transfer() {
            let _ = dt.set_data("text/plain", &path_drag);
        }
        drag_payload.set(Some(DragPayload::Folder(path_drag.clone())));
    };
    let on_dragover = move |e: DragEvent| {
        if drag_payload.get().is_some() {
            e.prevent_default();
            e.stop_propagation();
            drag_over.set(true);
        }
    };
    let on_dragleave = move |_| drag_over.set(false);
    let is_renaming = move || matches!(renaming.get(), Some(RenameTarget::Folder(ref p)) if *p == path_rename_check);

    view! {
        <li draggable="true" on:dragstart=on_dragstart>
            {move || {
                if is_renaming() {
                    let pr = path_rename.clone();
                    let nm = name_for_rename.clone();
                    view! {
                        <RenameInput
                            initial=nm
                            indent_style=indent_style.clone()
                            icon="folder"
                            on_confirm=Callback::new(move |new_name: String| {
                                let source = pr.clone();
                                renaming.set(None);
                                let old_name = source.rsplit('/').next().unwrap_or(&source).to_string();
                                if new_name == old_name || new_name.is_empty() {
                                    return;
                                }
                                leptos::task::spawn_local(async move {
                                    match ipc::rename_folder(&source, &new_name).await {
                                        Ok(()) => refresh_tree(notes, folders, error_msg).await,
                                        Err(e) => error_msg.set(Some(format!("Failed to rename folder: {e}"))),
                                    }
                                });
                            })
                            on_cancel=Callback::new(move |()| renaming.set(None))
                        />
                    }
                    .into_any()
                } else {
                    let path_ctx = path.clone();
                    let path_drop = path.clone();
                    view! {
                        <button
                            class=move || {
                                let base = "w-full text-left py-1 text-sm text-stone-400 hover:text-stone-200 transition-colors flex items-center gap-1";
                                if drag_over.get() { format!("{base} bg-stone-600/40") } else { base.to_string() }
                            }
                            style=indent_style.clone()
                            on:click=move |_| open.update(|v| *v = !*v)
                            on:contextmenu=move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();
                                context_menu.set(Some(ContextMenu {
                                    x: e.client_x(),
                                    y: e.client_y(),
                                    target: ContextTarget::Folder(path_ctx.clone()),
                                }));
                            }
                            on:dragover=on_dragover
                            on:dragleave=on_dragleave
                            on:drop=move |e: DragEvent| {
                                e.prevent_default();
                                e.stop_propagation();
                                drag_over.set(false);
                                if let Some(payload) = drag_payload.get() {
                                    drag_payload.set(None);
                                    handle_drop(
                                        payload,
                                        Some(path_drop.clone()),
                                        notes,
                                        folders,
                                        active_note,
                                        error_msg,
                                    );
                                }
                            }
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                class=move || {
                                    let base = "w-3 h-3 shrink-0 transition-transform";
                                    if open.get() { format!("{base} rotate-90") } else { base.to_string() }
                                }
                                fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3"
                            >
                                <path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
                            </svg>
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
                            </svg>
                            {name.clone()}
                        </button>
                    }
                    .into_any()
                }
            }}
            <ul class=move || if open.get() { "" } else { "hidden" }>
                {children_views}
            </ul>
        </li>
    }
}
