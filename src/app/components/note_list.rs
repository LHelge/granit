use leptos::prelude::*;

use crate::app::ipc;
use granit_types::{Note, NoteMeta};

// ── Tree model ─────────────────────────────────────────────────────

/// A node in the display tree built from flat `NoteMeta` list.
#[derive(Clone)]
enum TreeNode {
    Note(NoteMeta),
    Folder {
        name: String,
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
            children,
        });
    }
}

// ── Components ─────────────────────────────────────────────────────

#[component]
pub fn NoteList(
    notes: RwSignal<Vec<NoteMeta>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
    notes_error: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
        if let Some(err) = notes_error.get() {
            return view! {
                <p class="p-2 text-sm text-red-400 italic">{format!("Error loading notes: {err}")}</p>
            }
            .into_any();
        }
        let note_list = notes.get();
        if note_list.is_empty() {
            view! { <p class="p-2 text-sm text-stone-500 italic">"No notes yet"</p> }.into_any()
        } else {
            let tree = build_tree(note_list);
            view! {
                <ul class="py-1">
                    {tree.into_iter()
                        .map(|node| render_node(node, active_note, error_msg, 0))
                        .collect_view()}
                </ul>
            }
            .into_any()
        }
    }
}

fn render_node(
    node: TreeNode,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
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
            let on_click = move |_| {
                let s = slug_click.clone();
                leptos::task::spawn_local(async move {
                    match ipc::read_note(&s).await {
                        Ok(note) => active_note.set(Some(note)),
                        Err(e) => error_msg.set(Some(format!("Failed to load note: {e}"))),
                    }
                });
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
            }.into_any()
        }

        TreeNode::Folder { name, children } => {
            let open = RwSignal::new(true);
            // Build children eagerly; use a CSS "hidden" class instead of <Show> to
            // avoid ownership/Sync issues with passing rendered views into Fn closures.
            let children_views = children
                .into_iter()
                .map(|child| render_node(child, active_note, error_msg, depth + 1))
                .collect_view();
            view! {
                <li>
                    <button
                        class="w-full text-left py-1 text-sm text-stone-400 hover:text-stone-200 transition-colors flex items-center gap-1"
                        style=indent_style
                        on:click=move |_| open.update(|v| *v = !*v)
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
            }.into_any()
        }
    }
}
