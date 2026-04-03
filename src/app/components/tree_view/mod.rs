mod context_menu;
mod folder_node;
mod note_node;
mod rename_input;
mod tree_model;

use crate::app::{
    components::editor::{EditOpen, OpenInEdit},
    ipc,
};
use context_menu::TreeContextMenu;
use folder_node::FolderNode;
use granit_types::{Note, NoteMeta};
use leptos::prelude::*;
use note_node::NoteNode;
use tree_model::{build_tree, TreeNode};
use web_sys::{DragEvent, MouseEvent};

// ── Shared state via context ───────────────────────────────────────

/// Shared reactive state for the entire tree, provided via Leptos context
/// so child components can `use_context::<TreeCtx>()` instead of prop drilling.
#[derive(Clone, Copy)]
pub(super) struct TreeCtx {
    pub notes: RwSignal<Vec<NoteMeta>>,
    pub folders: RwSignal<Vec<String>>,
    pub active_note: RwSignal<Option<Note>>,
    app: crate::app::AppCtx,
    pub context_menu: RwSignal<Option<ContextMenu>>,
    pub drag_payload: RwSignal<Option<DragPayload>>,
    pub renaming: RwSignal<Option<RenameTarget>>,
    pub open_in_edit: RwSignal<EditOpen>,
}

impl TreeCtx {
    /// Push an error to the app-wide error channel.
    pub fn push_error(&self, msg: impl Into<String>) {
        self.app.push_error("tree", msg);
    }

    /// Process a drop event targeted at `dest_folder` (None = cave root).
    pub fn handle_drop(self, payload: DragPayload, dest_folder: Option<String>) {
        match payload {
            DragPayload::Note(slug) => {
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::move_note(&slug, dest_folder.as_deref()).await {
                        self.push_error(format!("Failed to move note: {e}"));
                        return;
                    }
                    if self
                        .active_note
                        .get()
                        .map(|n| n.meta.slug == slug)
                        .unwrap_or(false)
                    {
                        if let Ok(note) = ipc::read_note(&slug).await {
                            self.active_note.set(Some(note));
                        }
                    }
                    self.refresh_async().await;
                });
            }
            DragPayload::Folder(src_path) => {
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::move_folder(&src_path, dest_folder.as_deref()).await {
                        self.push_error(format!("Failed to move folder: {e}"));
                        return;
                    }
                    self.refresh_async().await;
                });
            }
        }
    }

    /// Async version of refresh (for use inside spawn_local blocks).
    async fn refresh_async(self) {
        match ipc::fetch_notes().await {
            Ok(list) => self.notes.set(list),
            Err(e) => {
                self.push_error(format!("Failed to refresh notes: {e}"));
                return;
            }
        }
        if let Ok(list) = ipc::fetch_folders().await {
            self.folders.set(list);
        }
    }
}

/// Retrieve the tree context from a child component.
pub(super) fn use_tree_ctx() -> TreeCtx {
    expect_context::<TreeCtx>()
}

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

// ── Render helper (dispatches to NoteNode / FolderNode) ────────────

fn render_node(node: TreeNode, depth: usize) -> impl IntoView {
    let indent_px = depth * 12;
    let indent_style = format!("padding-left: {}px", 12 + indent_px);

    match node {
        TreeNode::Note(meta) => {
            view! { <NoteNode meta=meta indent_style=indent_style /> }.into_any()
        }
        TreeNode::Folder {
            name,
            path,
            children,
        } => view! {
            <FolderNode
                name=name
                path=path
                children=children
                indent_style=indent_style
                depth=depth
            />
        }
        .into_any(),
    }
}

// ── Main component ─────────────────────────────────────────────────

#[component]
pub fn TreeView() -> impl IntoView {
    let app = expect_context::<crate::app::AppCtx>();
    let ctx = TreeCtx {
        notes: app.notes,
        folders: app.folders,
        active_note: app.active_note,
        app,
        context_menu: RwSignal::new(None),
        drag_payload: RwSignal::new(None),
        renaming: RwSignal::new(None),
        open_in_edit: expect_context::<OpenInEdit>().0,
    };
    provide_context(ctx);

    view! {
        <div
            class="relative min-h-full"
            on:contextmenu=move |e: MouseEvent| {
                e.prevent_default();
                ctx.context_menu.set(Some(ContextMenu {
                    x: e.client_x(),
                    y: e.client_y(),
                    target: ContextTarget::Root,
                }));
            }
            on:dragover=move |e: DragEvent| {
                if ctx.drag_payload.get().is_some() {
                    e.prevent_default();
                }
            }
            on:drop=move |e: DragEvent| {
                e.prevent_default();
                if let Some(payload) = ctx.drag_payload.get() {
                    ctx.drag_payload.set(None);
                    ctx.handle_drop(payload, None);
                }
            }
            on:dragend=move |_| {
                ctx.drag_payload.set(None);
            }
        >
            {move || {
                if let Some(err) = app.first_error_for("notes") {
                    return view! {
                        <p class="p-2 text-sm text-red-400 italic">
                            {format!("Error loading notes: {err}")}
                        </p>
                    }
                    .into_any();
                }
                let note_list = ctx.notes.get();
                let folder_list = ctx.folders.get();
                if note_list.is_empty() && folder_list.is_empty() {
                    view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }
                        .into_any()
                } else {
                    let tree = build_tree(note_list, folder_list);
                    view! {
                        <ul class="py-1">
                            {tree.into_iter().map(|node| render_node(node, 0)).collect_view()}
                        </ul>
                    }
                    .into_any()
                }
            }}
            <TreeContextMenu />
        </div>
    }
}
