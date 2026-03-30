use leptos::prelude::*;
use web_sys::MouseEvent;

use super::{use_tree_ctx, ContextTarget, RenameTarget};
use crate::app::ipc;

const MENU_ITEM: &str =
    "w-full text-left px-3 py-1.5 text-stone-200 hover:bg-stone-700 transition-colors";
const MENU_ITEM_DANGER: &str =
    "w-full text-left px-3 py-1.5 text-red-400 hover:bg-stone-700 transition-colors";

/// Renders the floating context menu overlay + panel.
#[component]
pub(super) fn TreeContextMenu() -> impl IntoView {
    let ctx = use_tree_ctx();

    move || {
        ctx.context_menu.get().map(|cm| {
            let x = cm.x;
            let y = cm.y;
            let target = cm.target.clone();
            view! {
                <div
                    class="fixed inset-0 z-40"
                    on:click=move |_| ctx.context_menu.set(None)
                    on:contextmenu=move |e: MouseEvent| {
                        e.prevent_default();
                        ctx.context_menu.set(None);
                    }
                />
                <div
                    class="fixed z-50 bg-stone-800 border border-stone-600 rounded shadow-lg py-1 min-w-40 text-sm"
                    style=format!("left:{x}px;top:{y}px")
                >
                    {match target {
                        ContextTarget::Note(slug) => render_note_menu(ctx, slug).into_any(),
                        ContextTarget::Folder(path) => render_folder_menu(ctx, path).into_any(),
                        ContextTarget::Root => render_root_menu(ctx).into_any(),
                    }}
                </div>
            }
        })
    }
}

fn render_note_menu(ctx: super::TreeCtx, slug: String) -> impl IntoView {
    let slug_rename = slug.clone();
    let slug_del = slug;
    view! {
        <button
            class=MENU_ITEM
            on:click=move |_| {
                ctx.context_menu.set(None);
                ctx.renaming.set(Some(RenameTarget::Note(slug_rename.clone())));
            }
        >
            "Rename"
        </button>
        <button
            class=MENU_ITEM_DANGER
            on:click=move |_| {
                let s = slug_del.clone();
                ctx.context_menu.set(None);
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::delete_note(&s).await {
                        ctx.error_msg.set(Some(format!("Failed to delete note: {e}")));
                        return;
                    }
                    if ctx.active_note.get().map(|n| n.meta.slug == s).unwrap_or(false) {
                        ctx.active_note.set(None);
                    }
                    ctx.refresh_async().await;
                });
            }
        >
            "Delete note"
        </button>
    }
}

fn render_folder_menu(ctx: super::TreeCtx, path: String) -> impl IntoView {
    let path_new_note = path.clone();
    let path_new_folder = path.clone();
    let path_rename = path.clone();
    let path_del = path;
    view! {
        <button
            class=MENU_ITEM
            on:click=move |_| {
                let p = path_new_note.clone();
                ctx.context_menu.set(None);
                leptos::task::spawn_local(async move {
                    match ipc::create_note("untitled", Some(&p)).await {
                        Ok(meta) => {
                            ctx.refresh_async().await;
                            match ipc::read_note(&meta.slug).await {
                                Ok(note) => ctx.active_note.set(Some(note)),
                                Err(e) => ctx.error_msg.set(Some(format!("Failed to open note: {e}"))),
                            }
                        }
                        Err(e) => ctx.error_msg.set(Some(format!("Failed to create note: {e}"))),
                    }
                });
            }
        >
            "New note here"
        </button>
        <button
            class=MENU_ITEM
            on:click=move |_| {
                let p = path_new_folder.clone();
                ctx.context_menu.set(None);
                leptos::task::spawn_local(async move {
                    let new_path = if p.is_empty() {
                        "new-folder".to_string()
                    } else {
                        format!("{p}/new-folder")
                    };
                    match ipc::create_folder(&new_path).await {
                        Ok(()) => {
                            ctx.refresh_async().await;
                            ctx.renaming.set(Some(RenameTarget::Folder(new_path)));
                        }
                        Err(e) => ctx.error_msg.set(Some(format!("Failed to create folder: {e}"))),
                    }
                });
            }
        >
            "New folder here"
        </button>
        <button
            class=MENU_ITEM
            on:click=move |_| {
                ctx.context_menu.set(None);
                ctx.renaming.set(Some(RenameTarget::Folder(path_rename.clone())));
            }
        >
            "Rename"
        </button>
        <button
            class=MENU_ITEM_DANGER
            on:click=move |_| {
                let p = path_del.clone();
                ctx.context_menu.set(None);
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::delete_folder(&p).await {
                        ctx.error_msg.set(Some(format!("Failed to delete folder: {e}")));
                        return;
                    }
                    if ctx
                        .active_note
                        .get()
                        .map(|n| n.meta.relative_path.starts_with(&format!("{p}/")))
                        .unwrap_or(false)
                    {
                        ctx.active_note.set(None);
                    }
                    ctx.refresh_async().await;
                });
            }
        >
            "Delete folder"
        </button>
    }
}

fn render_root_menu(ctx: super::TreeCtx) -> impl IntoView {
    view! {
        <button
            class=MENU_ITEM
            on:click=move |_| {
                ctx.context_menu.set(None);
                leptos::task::spawn_local(async move {
                    match ipc::create_folder("new-folder").await {
                        Ok(()) => {
                            ctx.refresh_async().await;
                            ctx.renaming.set(Some(RenameTarget::Folder("new-folder".to_string())));
                        }
                        Err(e) => ctx.error_msg.set(Some(format!("Failed to create folder: {e}"))),
                    }
                });
            }
        >
            "New folder"
        </button>
    }
}
