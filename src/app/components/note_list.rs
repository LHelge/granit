use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use crate::app::ipc;
use granit_types::{Note, NoteMeta};

// ── Tree model ─────────────────────────────────────────────────────

/// A node in the display tree built from flat `NoteMeta` list.
#[derive(Clone)]
enum TreeNode {
    Note(NoteMeta),
    Folder {
        name: String,
        /// Relative path from cave root, e.g. `"projects/2026"`.
        path: String,
        children: Vec<TreeNode>,
    },
}

/// Build a display tree from a flat list of NoteMeta and folder paths.
/// Each `relative_path` like `"a/b/note.md"` is split on `/` to produce the hierarchy.
/// `folders` ensures empty directories also appear in the tree.
fn build_tree(notes: Vec<NoteMeta>, folders: Vec<String>) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();

    // Ensure all folder paths exist in the tree (including empty ones).
    let mut sorted_folders = folders;
    sorted_folders.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    for folder_path in sorted_folders {
        let parts: Vec<&str> = folder_path.split('/').collect();
        ensure_folder(&mut roots, &parts, 0);
    }

    // Sort so folders and notes appear deterministically.
    let mut sorted = notes;
    sorted.sort_by(|a, b| {
        a.relative_path
            .to_lowercase()
            .cmp(&b.relative_path.to_lowercase())
    });

    for meta in sorted {
        let relative_path = meta.relative_path.clone();
        let parts: Vec<&str> = relative_path.split('/').collect();
        insert_node(&mut roots, &parts, 0, meta);
    }

    roots
}

/// Ensure a folder path exists in the tree, creating empty folder nodes as needed.
fn ensure_folder(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize) {
    if depth >= parts.len() {
        return;
    }
    let folder_name = parts[depth].to_string();
    let folder_path = parts[0..=depth].join("/");
    if let Some(TreeNode::Folder { children, .. }) = nodes
        .iter_mut()
        .find(|n| matches!(n, TreeNode::Folder { name, .. } if *name == folder_name))
    {
        ensure_folder(children, parts, depth + 1);
    } else {
        let mut children = Vec::new();
        ensure_folder(&mut children, parts, depth + 1);
        nodes.push(TreeNode::Folder {
            name: folder_name,
            path: folder_path,
            children,
        });
    }
}

fn insert_node(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize, meta: NoteMeta) {
    if depth == parts.len().saturating_sub(1) {
        // Leaf — a note.
        nodes.push(TreeNode::Note(meta));
        return;
    }
    // Intermediate — a folder.
    let folder_name = parts[depth].to_string();
    let folder_path = parts[0..=depth].join("/");
    if let Some(TreeNode::Folder { children, .. }) = nodes
        .iter_mut()
        .find(|n| matches!(n, TreeNode::Folder { name, .. } if *name == folder_name))
    {
        insert_node(children, parts, depth + 1, meta);
    } else {
        let mut children = Vec::new();
        insert_node(&mut children, parts, depth + 1, meta);
        nodes.push(TreeNode::Folder {
            name: folder_name,
            path: folder_path,
            children,
        });
    }
}

// ── Drag-and-drop helpers ──────────────────────────────────────────

/// What is being dragged: a note (by slug) or a folder (by relative path).
#[derive(Clone, Debug)]
enum DragPayload {
    Note(String),
    Folder(String),
}

/// Refresh both the notes list and the folder list.
async fn refresh_tree(
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
fn handle_drop(
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

// ── Context menu ───────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum ContextTarget {
    Note(String),   // slug
    Folder(String), // relative path from cave root
    Root,           // cave root (empty area)
}

#[derive(Clone, Debug)]
struct ContextMenu {
    x: i32,
    y: i32,
    target: ContextTarget,
}

/// What is currently being renamed inline.
#[derive(Clone, Debug)]
enum RenameTarget {
    Note(String),   // slug
    Folder(String), // relative path
}

// ── Components ─────────────────────────────────────────────────────

#[component]
pub fn NoteList(
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
                // Root-level right-click — show "New folder" at cave root.
                e.prevent_default();
                context_menu.set(Some(ContextMenu {
                    x: e.client_x(),
                    y: e.client_y(),
                    target: ContextTarget::Root,
                }));
            }
            on:dragover=move |e: DragEvent| {
                // Allow drop anywhere — this is the cave-root drop target.
                // Folder drop handlers call stop_propagation so this only fires
                // when the cursor is NOT over a folder.
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
            // Tree — re-renders when notes or notes_error changes
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
            // Context menu — re-renders only when context_menu signal changes
            {move || {
                context_menu.get().map(|cm| {
                    let x = cm.x;
                    let y = cm.y;
                    let target = cm.target.clone();
                    view! {
                        // Transparent overlay — click outside to dismiss
                        <div
                            class="fixed inset-0 z-40"
                            on:click=move |_| context_menu.set(None)
                            on:contextmenu=move |e: MouseEvent| {
                                e.prevent_default();
                                context_menu.set(None);
                            }
                        />
                        // Floating menu panel
                        <div
                            class="fixed z-50 bg-stone-800 border border-stone-600 rounded shadow-lg py-1 min-w-40 text-sm"
                            style=format!("left:{x}px;top:{y}px")
                        >
                            {match target {
                                ContextTarget::Note(slug) => {
                                    let slug_rename = slug.clone();
                                    let slug_del = slug.clone();
                                    view! {
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                context_menu.set(None);
                                                renaming.set(Some(RenameTarget::Note(slug_rename.clone())));
                                            }
                                        >
                                            "Rename"
                                        </button>
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-red-400 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                let s = slug_del.clone();
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    if let Err(e) = ipc::delete_note(&s).await {
                                                        error_msg.set(Some(format!("Failed to delete note: {e}")));
                                                        return;
                                                    }
                                                    if active_note
                                                        .get()
                                                        .map(|n| n.meta.slug == s)
                                                        .unwrap_or(false)
                                                    {
                                                        active_note.set(None);
                                                    }
                                                    refresh_tree(notes, folders, error_msg).await;
                                                });
                                            }
                                        >
                                            "Delete note"
                                        </button>
                                    }
                                    .into_any()
                                }
                                ContextTarget::Folder(path) => {
                                    let path_new_note = path.clone();
                                    let path_new_folder = path.clone();
                                    let path_rename = path.clone();
                                    let path_del = path.clone();
                                    view! {
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                let p = path_new_note.clone();
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    match ipc::create_note("untitled", Some(&p)).await {
                                                        Ok(meta) => {
                                                            refresh_tree(notes, folders, error_msg).await;
                                                            match ipc::read_note(&meta.slug).await {
                                                                Ok(note) => active_note.set(Some(note)),
                                                                Err(e) => error_msg.set(Some(
                                                                    format!("Failed to open note: {e}"),
                                                                )),
                                                            }
                                                        }
                                                        Err(e) => error_msg.set(Some(format!(
                                                            "Failed to create note: {e}"
                                                        ))),
                                                    }
                                                });
                                            }
                                        >
                                            "New note here"
                                        </button>
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                let p = path_new_folder.clone();
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    let new_path = if p.is_empty() {
                                                        "new-folder".to_string()
                                                    } else {
                                                        format!("{p}/new-folder")
                                                    };
                                                    match ipc::create_folder(&new_path).await {
                                                        Ok(()) => {
                                                            refresh_tree(notes, folders, error_msg).await;
                                                            renaming.set(Some(RenameTarget::Folder(new_path)));
                                                        }
                                                        Err(e) => error_msg.set(Some(format!(
                                                            "Failed to create folder: {e}"
                                                        ))),
                                                    }
                                                });
                                            }
                                        >
                                            "New folder here"
                                        </button>
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                context_menu.set(None);
                                                renaming.set(Some(RenameTarget::Folder(path_rename.clone())));
                                            }
                                        >
                                            "Rename"
                                        </button>
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-red-400 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                let p = path_del.clone();
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    if let Err(e) = ipc::delete_folder(&p).await {
                                                        error_msg.set(Some(format!(
                                                            "Failed to delete folder: {e}"
                                                        )));
                                                        return;
                                                    }
                                                    if active_note
                                                        .get()
                                                        .map(|n| {
                                                            n.meta
                                                                .relative_path
                                                                .starts_with(&format!("{p}/"))
                                                        })
                                                        .unwrap_or(false)
                                                    {
                                                        active_note.set(None);
                                                    }
                                                    refresh_tree(notes, folders, error_msg).await;
                                                });
                                            }
                                        >
                                            "Delete folder"
                                        </button>
                                    }
                                    .into_any()
                                }
                                ContextTarget::Root => {
                                    view! {
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    match ipc::create_folder("new-folder").await {
                                                        Ok(()) => {
                                                            refresh_tree(notes, folders, error_msg).await;
                                                            renaming.set(Some(RenameTarget::Folder("new-folder".to_string())));
                                                        }
                                                        Err(e) => error_msg.set(Some(format!(
                                                            "Failed to create folder: {e}"
                                                        ))),
                                                    }
                                                });
                                            }
                                        >
                                            "New folder"
                                        </button>
                                    }
                                    .into_any()
                                }
                            }}
                        </div>
                    }
                })
            }}
        </div>
    }
}

// `notes` is passed recursively AND captured by context-menu closures inside the
// `view!` macro — clippy can't see the latter, so suppress the false positive.
#[allow(clippy::only_used_in_recursion, clippy::too_many_arguments)]
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
        TreeNode::Note(meta) => {
            let slug = meta.slug.clone();
            let slug_drag = meta.slug.clone();
            let slug_rename = meta.slug.clone();
            let slug_rename_check = meta.slug.clone();
            let display = meta.slug.clone();
            let is_renaming = move || matches!(renaming.get(), Some(RenameTarget::Note(ref s)) if *s == slug_rename_check);
            let on_dragstart = move |e: DragEvent| {
                e.stop_propagation();
                drag_payload.set(Some(DragPayload::Note(slug_drag.clone())));
            };
            view! {
                <li
                    draggable="true"
                    on:dragstart=on_dragstart
                >
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
                            }.into_any()
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
                            }.into_any()
                        }
                    }}
                </li>
            }
            .into_any()
        }

        TreeNode::Folder {
            name,
            path,
            children,
        } => {
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
                <li
                    draggable="true"
                    on:dragstart=on_dragstart
                >
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
                            }.into_any()
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
                            }.into_any()
                        }
                    }}
                    <ul class=move || if open.get() { "" } else { "hidden" }>
                        {children_views}
                    </ul>
                </li>
            }
            .into_any()
        }
    }
}

/// Inline rename input component. Shows a text input with the current name,
/// commits on Enter, cancels on Escape or blur.
#[component]
fn RenameInput(
    initial: String,
    indent_style: String,
    /// "note" or "folder" — determines which icon to show.
    icon: &'static str,
    on_confirm: Callback<String>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (text, set_text) = signal(initial);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Auto-focus after mount.
    Effect::new(move || {
        if let Some(el) = input_ref.get() {
            let el: &web_sys::HtmlInputElement = &el;
            let _ = el.focus();
            el.select();
        }
    });

    let on_keydown = move |e: KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            e.prevent_default();
            on_confirm.run(text.get().trim().to_string());
        }
        "Escape" => {
            e.prevent_default();
            on_cancel.run(());
        }
        _ => {}
    };

    let on_blur = move |_| {
        on_confirm.run(text.get().trim().to_string());
    };

    let note_icon = icon == "note";

    view! {
        <div class="flex items-center gap-1 py-0.5 text-sm" style=indent_style>
            {if note_icon {
                view! {
                    <span class="w-3 shrink-0" />
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0 text-stone-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                }.into_any()
            } else {
                view! {
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 shrink-0 text-stone-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
                    </svg>
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
                    </svg>
                }.into_any()
            }}
            <input
                type="text"
                prop:value=move || text.get()
                node_ref=input_ref
                class="flex-1 bg-stone-700 text-stone-100 text-sm px-1 py-0 rounded border border-stone-500 focus:outline-none focus:border-stone-400 min-w-0"
                on:input=move |ev| set_text.set(event_target_value(&ev))
                on:keydown=on_keydown
                on:blur=on_blur
            />
        </div>
    }
}
