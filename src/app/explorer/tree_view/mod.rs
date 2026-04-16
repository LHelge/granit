mod context_menu;
mod folder_node;
mod note_node;
mod rename_input;
mod tree_model;

use crate::app::{
    editor::{EditOpen, OpenInEdit},
    ipc,
};
use context_menu::TreeContextMenu;
use folder_node::FolderNode;
use granit_types::{Document, DocumentMeta};
use leptos::prelude::*;
use note_node::NoteNode;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use tree_model::{build_tree, TreeNode};
use web_sys::{DragEvent, MouseEvent};

// ── Shared state via context ───────────────────────────────────────

/// A pending tree mutation that must be serialized against other tree ops
/// so rapid drag-and-drop events cannot race their backend effects or
/// the subsequent listing refresh.
#[derive(Clone, Debug)]
enum TreeOp {
    MoveNote { slug: String, dest: Option<String> },
    MoveFolder { src: String, dest: Option<String> },
}

/// FIFO queue that guarantees at most one tree op is in flight at a time.
/// Ops are executed in submission order; after the queue drains, a single
/// refresh is triggered by the last op.
struct TreeOpQueue {
    pending: VecDeque<TreeOp>,
    in_flight: bool,
}

impl TreeOpQueue {
    fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_flight: false,
        }
    }

    /// Push an op. Returns `true` if the caller should start the drain loop.
    fn enqueue(&mut self, op: TreeOp) -> bool {
        self.pending.push_back(op);
        if self.in_flight {
            false
        } else {
            self.in_flight = true;
            true
        }
    }

    fn take_next(&mut self) -> Option<TreeOp> {
        match self.pending.pop_front() {
            Some(op) => Some(op),
            None => {
                self.in_flight = false;
                None
            }
        }
    }
}

/// Shared reactive state for the entire tree, provided via Leptos context
/// so child components can `use_context::<TreeCtx>()` instead of prop drilling.
#[derive(Clone, Copy)]
pub(super) struct TreeCtx {
    pub notes: RwSignal<Vec<DocumentMeta>>,
    pub folders: RwSignal<Vec<String>>,
    pub active_note: RwSignal<Option<Document>>,
    app: crate::app::AppCtx,
    pub context_menu: RwSignal<Option<ContextMenu>>,
    pub drag_payload: RwSignal<Option<DragPayload>>,
    pub renaming: RwSignal<Option<RenameTarget>>,
    pub open_in_edit: RwSignal<EditOpen>,
    /// Serializes tree mutations (moves) so drops cannot race each other.
    op_queue: StoredValue<Rc<RefCell<TreeOpQueue>>, LocalStorage>,
}

impl TreeCtx {
    /// Push an error to the app-wide error channel.
    pub fn push_error(&self, msg: impl Into<String>) {
        self.app.push_error("tree", msg);
    }

    /// Process a drop event targeted at `dest_folder` (None = cave root).
    pub fn handle_drop(self, payload: DragPayload, dest_folder: Option<String>) {
        let op = match payload {
            DragPayload::Note(slug) => TreeOp::MoveNote {
                slug,
                dest: dest_folder,
            },
            DragPayload::Folder(src_path) => TreeOp::MoveFolder {
                src: src_path,
                dest: dest_folder,
            },
        };
        let should_drive = self.op_queue.with_value(|q| q.borrow_mut().enqueue(op));
        if should_drive {
            leptos::task::spawn_local(async move {
                self.drain_ops().await;
            });
        }
    }

    /// Drain loop: execute queued ops one at a time, then refresh once
    /// after the queue empties.
    async fn drain_ops(self) {
        loop {
            let next = self.op_queue.with_value(|q| q.borrow_mut().take_next());
            let Some(op) = next else {
                break;
            };
            match op {
                TreeOp::MoveNote { slug, dest } => {
                    if let Err(e) = ipc::move_note(&slug, dest.as_deref()).await {
                        self.push_error(format!("Failed to move note: {e}"));
                        continue;
                    }
                    if self
                        .active_note
                        .get()
                        .map(|n| n.meta.slug == slug)
                        .unwrap_or(false)
                    {
                        if let Ok(note) = ipc::read_note(&slug).await {
                            self.app.set_active_note_document(note);
                        }
                    }
                }
                TreeOp::MoveFolder { src, dest } => {
                    if let Err(e) = ipc::move_folder(&src, dest.as_deref()).await {
                        self.push_error(format!("Failed to move folder: {e}"));
                        continue;
                    }
                }
            }
        }
        // Single refresh after the queue drains; avoids N refreshes for N drops.
        self.refresh_async().await;
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
        op_queue: StoredValue::new_local(Rc::new(RefCell::new(TreeOpQueue::new()))),
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
                        <p class="p-2 text-sm text-error italic">
                            {format!("Error loading notes: {err}")}
                        </p>
                    }
                    .into_any();
                }
                let note_list = ctx.notes.get();
                let folder_list = ctx.folders.get();
                if note_list.is_empty() && folder_list.is_empty() {
                    view! { <p class="p-2 text-sm text-base-content/35 italic">"No notes yet"</p> }
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
