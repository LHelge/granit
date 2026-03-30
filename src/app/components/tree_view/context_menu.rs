use leptos::prelude::*;
use web_sys::MouseEvent;

use granit_types::{Note, NoteMeta};

use super::{refresh_tree, ContextMenu, ContextTarget, RenameTarget};
use crate::app::ipc;

/// Renders the floating context menu overlay + panel.
#[component]
pub(super) fn TreeContextMenu(
    context_menu: RwSignal<Option<ContextMenu>>,
    renaming: RwSignal<Option<RenameTarget>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
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
                        ContextTarget::Note(slug) => render_note_menu(slug, context_menu, renaming, notes, folders, active_note, error_msg).into_any(),
                        ContextTarget::Folder(path) => render_folder_menu(path, context_menu, renaming, notes, folders, active_note, error_msg).into_any(),
                        ContextTarget::Root => render_root_menu(context_menu, renaming, notes, folders, error_msg).into_any(),
                    }}
                </div>
            }
        })
    }
}

fn render_note_menu(
    slug: String,
    context_menu: RwSignal<Option<ContextMenu>>,
    renaming: RwSignal<Option<RenameTarget>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
) -> impl IntoView {
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

#[allow(clippy::too_many_arguments)]
fn render_folder_menu(
    path: String,
    context_menu: RwSignal<Option<ContextMenu>>,
    renaming: RwSignal<Option<RenameTarget>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    active_note: RwSignal<Option<Note>>,
    error_msg: RwSignal<Option<String>>,
) -> impl IntoView {
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

fn render_root_menu(
    context_menu: RwSignal<Option<ContextMenu>>,
    renaming: RwSignal<Option<RenameTarget>>,
    notes: RwSignal<Vec<NoteMeta>>,
    folders: RwSignal<Vec<String>>,
    error_msg: RwSignal<Option<String>>,
) -> impl IntoView {
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
