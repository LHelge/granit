mod context_menu;
mod folder_node;
mod note_node;
mod rename_input;
mod tree_model;

use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use granit_types::{Note, NoteMeta};

use context_menu::TreeContextMenu;
use folder_node::FolderNode;
use note_node::NoteNode;
use tree_model::{build_tree, TreeNode};

use crate::app::ipc;

// ── Shared types ───────────────────────────────────────────────────

/// What is being dragged: a note (by slug) or a folder (by relative path).
#[derive(Clone, Debug)]
pub(super) enum DragPayload {
    Note(String),
    Folder(String),
}

/// The target of a right-click context menu action.
#[derive(Clone, Debug)]
pub(super) enum ContextTarget {
    Note(String),   // slug
    Folder(String), // relative path from cave root
    Root,           // cave root (empty area)
}

#[derive(Clone, Debug)]
pub(super) struct ContextMenu {
    pub x: i32,
    pub y: i32,
    pub target: ContextTarget,
}

/// What is currently being renamed inline.
#[derive(Clone, Debug)]
pub(super) enum RenameTarget {
    Note(String),   // slug
    Folder(String), // relative path
}

// ── Shared helpers ─────────────────────────────────────────────────

/// Refresh both the notes list and the folder list.
pub(super) async fn refresh_tree(
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    error_msg: RwSignal<Option<String>>,
) {
    match ipc::fetch_notes().await {
        Ok(list) => notes.set(list),
        Err(e) => {
            error_msg.set(Some(format!("Failed to refresh notes: {e}")));
            return;
        }
    }
    if let Ok(list) = ipc::fetch_folders().await {
        folders.set(list);
    }
}

/// Process a drop event targeted at `dest_folder` (None = cave root).
pub(super) fn handle_drop(
    payload: DragPayload,
    dest_folder: Option<String>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
) {
    match payload {
        DragPayload::Note(slug) => {
            let dest = dest_folder.clone();
            leptos::task::spawn_local(async move {
                if let Err(e) = ipc::move_note(&slug, dest.as_deref()).await {
                    error_msg.set(Some(format!("Failed to move note: {e}")));
                    return;
                }
                if active_note
                    .get()
                    .map(|n| n.meta.slug == slug)
                    .unwrap_or(false)
                {
                    if let Ok(note) = ipc::read_note(&slug).await {
                        active_note.set(Some(note));
                    }
                }
                refresh_tree(notes, folders, error_msg).await;
            });
        }
        DragPayload::Folder(src_path) => {
            let dest = dest_folder.clone();
            leptos::task::spawn_local(async move {
                if let Err(e) = ipc::move_folder(&src_path, dest.as_deref()).await {
                    error_msg.set(Some(format!("Failed to move folder: {e}")));
                    return;
                }
                refresh_tree(notes, folders, error_msg).await;
            });
        }
    }
}

// ── Render helper (dispatches to NoteNode / FolderNode) ────────────

#[allow(clippy::too_many_arguments)]
fn render_node(
    node: TreeNode,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    context_menu: RwSignal<Option<ContextMenu>>,
    drag_payload: RwSignal<Option<DragPayload>>,
    renaming: RwSignal<Option<RenameTarget>>,
    depth: usize,
) -> impl IntoView {
    let indent_px = depth * 12;
    let indent_style = format!("padding-left: {}px", 12 + indent_px);

    match node {
        TreeNode::Note(meta) => view! {
            <NoteNode
                meta=meta
                active_note=active_note
                error_msg=error_msg
                notes=notes
                folders=folders
                context_menu=context_menu
                drag_payload=drag_payload
                renaming=renaming
                indent_style=indent_style
            />
        }
        .into_any(),

        TreeNode::Folder {
            name,
            path,
            children,
        } => view! {
            <FolderNode
                name=name
                path=path
                children=children
                active_note=active_note
                error_msg=error_msg
                notes=notes
                folders=folders
                context_menu=context_menu
                drag_payload=drag_payload
                renaming=renaming
                indent_style=indent_style
                depth=depth
            />
        }
        .into_any(),
    }
}

// ── Main component ─────────────────────────────────────────────────

#[component]
pub fn TreeView(
    notes: RwSignal<Vec<NoteMeta>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let context_menu: RwSignal<Option<ContextMenu>> = RwSignal::new(None);
    let drag_payload: RwSignal<Option<DragPayload>> = RwSignal::new(None);
    let folders: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let renaming: RwSignal<Option<RenameTarget>> = RwSignal::new(None);

    // Fetch folder list on mount.
    leptos::task::spawn_local(async move {
        if let Ok(list) = ipc::fetch_folders().await {
            folders.set(list);
        }
    });

    view! {
        <div
            class="relative min-h-full"
            on:contextmenu=move |e: MouseEvent| {
                e.prevent_default();
                context_menu.set(Some(ContextMenu {
                    x: e.client_x(),
                    y: e.client_y(),
                    target: ContextTarget::Root,
                }));
            }
            on:dragover=move |e: DragEvent| {
                if drag_payload.get().is_some() {
                    e.prevent_default();
                }
            }
            on:drop=move |e: DragEvent| {
                e.prevent_default();
                if let Some(payload) = drag_payload.get() {
                    drag_payload.set(None);
                    handle_drop(payload, None, notes, folders, active_note, error_msg);
                }
            }
            on:dragend=move |_| {
                drag_payload.set(None);
            }
        >
            {move || {
                if let Some(err) = notes_error.get() {
                    return view! {
                        <p class="p-2 text-sm text-red-400 italic">
                            {format!("Error loading notes: {err}")}
                        </p>
                    }
                    .into_any();
                }
                let note_list = notes.get();
                let folder_list = folders.get();
                if note_list.is_empty() && folder_list.is_empty() {
                    view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }
                        .into_any()
                } else {
                    let tree = build_tree(note_list, folder_list);
                    view! {
                        <ul class="py-1">
                            {tree
                                .into_iter()
                                .map(|node| {
                                    render_node(node, active_note, error_msg, notes, folders, context_menu, drag_payload, renaming, 0)
                                })
                                .collect_view()}
                        </ul>
                    }
                    .into_any()
                }
            }}
            <TreeContextMenu
                context_menu=context_menu
                renaming=renaming
                notes=notes
                folders=folders
                active_note=active_note
                error_msg=error_msg
            />
        </div>
    }
}
