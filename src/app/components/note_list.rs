use leptos::prelude::*;
use web_sys::MouseEvent;

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

/// Build a display tree from a flat list of NoteMeta.
/// Each `relative_path` like `"a/b/note.md"` is split on `/` to produce the hierarchy.
fn build_tree(notes: Vec<NoteMeta>) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();

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

// ── Context menu ───────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum ContextTarget {
    Note(String),   // slug
    Folder(String), // relative path from cave root
}

#[derive(Clone, Debug)]
struct ContextMenu {
    x: i32,
    y: i32,
    target: ContextTarget,
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

    view! {
        <div class="relative">
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
                if note_list.is_empty() {
                    view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }
                        .into_any()
                } else {
                    let tree = build_tree(note_list);
                    view! {
                        <ul class="py-1">
                            {tree
                                .into_iter()
                                .map(|node| {
                                    render_node(node, active_note, error_msg, notes, context_menu, 0)
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
                                    let slug_del = slug.clone();
                                    view! {
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
                                                    match ipc::fetch_notes().await {
                                                        Ok(list) => notes.set(list),
                                                        Err(e) => error_msg.set(Some(format!("Failed to refresh notes: {e}"))),
                                                    }
                                                });
                                            }
                                        >
                                            "Delete note"
                                        </button>
                                    }
                                    .into_any()
                                }
                                ContextTarget::Folder(path) => {
                                    let path_new = path.clone();
                                    let path_del = path.clone();
                                    view! {
                                        <button
                                            class="w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors"
                                            on:click=move |_| {
                                                let p = path_new.clone();
                                                context_menu.set(None);
                                                leptos::task::spawn_local(async move {
                                                    match ipc::create_note("untitled", Some(&p)).await {
                                                        Ok(meta) => {
                                                            match ipc::fetch_notes().await {
                                                                Ok(list) => notes.set(list),
                                                                Err(e) => {
                                                                    error_msg.set(Some(format!("Failed to refresh notes: {e}")));
                                                                    return;
                                                                }
                                                            }
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
                                                    // Clear active note if it was inside the deleted folder
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
                                                    match ipc::fetch_notes().await {
                                                        Ok(list) => notes.set(list),
                                                        Err(e) => error_msg.set(Some(format!(
                                                            "Failed to refresh notes: {e}"
                                                        ))),
                                                    }
                                                });
                                            }
                                        >
                                            "Delete folder"
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
#[allow(clippy::only_used_in_recursion)]
fn render_node(
    node: TreeNode,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes: RwSignal<Vec<NoteMeta>>,
    context_menu: RwSignal<Option<ContextMenu>>,
    depth: usize,
) -> impl IntoView {
    let indent_px = depth * 12;
    let indent_style = format!("padding-left: {}px", 12 + indent_px);

    match node {
        TreeNode::Note(meta) => {
            let slug = meta.slug.clone();
            let display = meta.slug.clone();
            let is_active = move || {
                active_note
                    .get()
                    .map(|n| n.meta.slug == slug)
                    .unwrap_or(false)
            };
            let slug_click = meta.slug.clone();
            let slug_ctx = meta.slug.clone();
            let on_click = move |_| {
                let s = slug_click.clone();
                leptos::task::spawn_local(async move {
                    match ipc::read_note(&s).await {
                        Ok(note) => active_note.set(Some(note)),
                        Err(e) => error_msg.set(Some(format!("Failed to load note: {e}"))),
                    }
                });
            };
            let on_contextmenu = move |e: MouseEvent| {
                e.prevent_default();
                context_menu.set(Some(ContextMenu {
                    x: e.client_x(),
                    y: e.client_y(),
                    target: ContextTarget::Note(slug_ctx.clone()),
                }));
            };
            view! {
                <li>
                    <button
                        class=move || {
                            let base = "w-full text-left py-1 text-sm truncate transition-colors flex items-center gap-1";
                            if is_active() {
                                format!("{base} bg-stone-700 text-stone-100")
                            } else {
                                format!("{base} text-stone-300 hover:bg-stone-700/50")
                            }
                        }
                        style=indent_style.clone()
                        on:click=on_click
                        on:contextmenu=on_contextmenu
                    >
                        // Spacer matching the chevron width so the note icon aligns with the folder icon
                        <span class="w-3 shrink-0" />
                        // Note icon
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0 text-stone-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                        </svg>
                        {display}
                    </button>
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
            let children_views = children
                .into_iter()
                .map(|child| {
                    render_node(
                        child,
                        active_note,
                        error_msg,
                        notes,
                        context_menu,
                        depth + 1,
                    )
                })
                .collect_view();
            let on_contextmenu = move |e: MouseEvent| {
                e.prevent_default();
                context_menu.set(Some(ContextMenu {
                    x: e.client_x(),
                    y: e.client_y(),
                    target: ContextTarget::Folder(path.clone()),
                }));
            };
            view! {
                <li>
                    <button
                        class="w-full text-left py-1 text-sm text-stone-400 hover:text-stone-200 transition-colors flex items-center gap-1"
                        style=indent_style
                        on:click=move |_| open.update(|v| *v = !*v)
                        on:contextmenu=on_contextmenu
                    >
                        // Chevron
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
                        // Folder icon
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
                        </svg>
                        {name}
                    </button>
                    <ul class=move || if open.get() { "" } else { "hidden" }>
                        {children_views}
                    </ul>
                </li>
            }
            .into_any()
        }
    }
}
