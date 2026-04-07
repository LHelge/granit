use super::{use_tree_ctx, ContextTarget, RenameTarget};
use crate::app::{editor::EditOpen, ipc};
use leptos::prelude::*;
use web_sys::MouseEvent;

fn spawn_create_note(ctx: super::TreeCtx, folder: Option<String>, template: Option<String>) {
    leptos::task::spawn_local(async move {
        match ipc::create_note("untitled", folder.as_deref(), template.as_deref()).await {
            Ok(meta) => {
                ctx.refresh_async().await;
                match ipc::read_note(&meta.slug).await {
                    Ok(note) => {
                        ctx.open_in_edit.set(EditOpen::EditFocusTitle);
                        ctx.app.set_active_note_document(note);
                    }
                    Err(e) => ctx.push_error(format!("Failed to open note: {e}")),
                }
            }
            Err(e) => ctx.push_error(format!("Failed to create note: {e}")),
        }
    });
}

fn render_new_note_actions(
    ctx: super::TreeCtx,
    folder: Option<String>,
    note_label: &'static str,
    template_label: &'static str,
) -> impl IntoView {
    let folder_new_note = folder.clone();
    let folder_from_template = folder;

    view! {
        <li>
            <button
                on:click=move |_| {
                    ctx.context_menu.set(None);
                    spawn_create_note(ctx, folder_new_note.clone(), None);
                }
            >
                {note_label}
            </button>
        </li>
        <li>
            <details>
                <summary>{template_label}</summary>
                <ul>
                    {move || {
                        let daily_template_slug = ctx.app.config.get().daily_note_template_slug;
                        let templates = ctx
                            .app
                            .templates
                            .get()
                            .into_iter()
                            .filter(|template| {
                                daily_template_slug
                                    .as_deref()
                                    .map(|slug| template.slug != slug)
                                    .unwrap_or(true)
                            })
                            .collect::<Vec<_>>();

                        if templates.is_empty() {
                            return view! {
                                <li>
                                    <button disabled=true class="text-base-content/35">
                                        "No templates yet"
                                    </button>
                                </li>
                            }
                            .into_any();
                        }

                        templates
                            .into_iter()
                            .map(|template| {
                                let target_folder = folder_from_template.clone();
                                let template_slug = template.slug.clone();
                                let template_label = template.slug;
                                view! {
                                    <li>
                                        <button
                                            on:click=move |_| {
                                                ctx.context_menu.set(None);
                                                spawn_create_note(
                                                    ctx,
                                                    target_folder.clone(),
                                                    Some(template_slug.clone()),
                                                );
                                            }
                                        >
                                            {template_label.clone()}
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </ul>
            </details>
        </li>
    }
}

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
                <ul
                    class="menu menu-sm fixed z-50 bg-base-300 border border-base-content/20 rounded shadow-lg p-1 min-w-40"
                    style=format!("left:{x}px;top:{y}px")
                >
                    {match target {
                        ContextTarget::Note(slug) => render_note_menu(ctx, slug).into_any(),
                        ContextTarget::Folder(path) => render_folder_menu(ctx, path).into_any(),
                        ContextTarget::Root => render_root_menu(ctx).into_any(),
                    }}
                </ul>
            }
        })
    }
}

fn render_note_menu(ctx: super::TreeCtx, slug: String) -> impl IntoView {
    let slug_rename = slug.clone();
    let slug_del = slug;
    view! {
        <li>
            <button
                on:click=move |_| {
                    ctx.context_menu.set(None);
                    ctx.renaming.set(Some(RenameTarget::Note(slug_rename.clone())));
                }
            >
                "Rename"
            </button>
        </li>
        <li>
            <button
                class="text-error"
                on:click=move |_| {
                    let s = slug_del.clone();
                    ctx.context_menu.set(None);
                    leptos::task::spawn_local(async move {
                        if let Err(e) = ipc::delete_note(&s).await {
                            ctx.push_error(format!("Failed to delete note: {e}"));
                            return;
                        }
                        if ctx.active_note.get().map(|n| n.meta.slug == s).unwrap_or(false) {
                            ctx.app.clear_active_document();
                        }
                        ctx.refresh_async().await;
                    });
                }
            >
                "Delete note"
            </button>
        </li>
    }
}

fn render_folder_menu(ctx: super::TreeCtx, path: String) -> impl IntoView {
    let path_new_folder = path.clone();
    let path_rename = path.clone();
    let path_del = path;
    view! {
        {render_new_note_actions(ctx, Some(path_new_folder.clone()), "New note here", "New note here from template")}
        <li>
            <button
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
                            Err(e) => ctx.push_error(format!("Failed to create folder: {e}")),
                        }
                    });
                }
            >
                "New folder here"
            </button>
        </li>
        <li>
            <button
                on:click=move |_| {
                    ctx.context_menu.set(None);
                    ctx.renaming.set(Some(RenameTarget::Folder(path_rename.clone())));
                }
            >
                "Rename"
            </button>
        </li>
        <li>
            <button
                class="text-error"
                on:click=move |_| {
                    let p = path_del.clone();
                    ctx.context_menu.set(None);
                    leptos::task::spawn_local(async move {
                        if let Err(e) = ipc::delete_folder(&p).await {
                            ctx.push_error(format!("Failed to delete folder: {e}"));
                            return;
                        }
                        if ctx
                            .active_note
                            .get()
                            .map(|n| n.meta.relative_path.starts_with(&format!("{p}/")))
                            .unwrap_or(false)
                        {
                            ctx.app.clear_active_document();
                        }
                        ctx.refresh_async().await;
                    });
                }
            >
                "Delete folder"
            </button>
        </li>
    }
}

fn render_root_menu(ctx: super::TreeCtx) -> impl IntoView {
    view! {
        {render_new_note_actions(ctx, None, "New note", "New note from template")}
        <li>
            <button
                on:click=move |_| {
                    ctx.context_menu.set(None);
                    leptos::task::spawn_local(async move {
                        match ipc::create_folder("new-folder").await {
                            Ok(()) => {
                                ctx.refresh_async().await;
                                ctx.renaming.set(Some(RenameTarget::Folder("new-folder".to_string())));
                            }
                            Err(e) => ctx.push_error(format!("Failed to create folder: {e}")),
                        }
                    });
                }
            >
                "New folder"
            </button>
        </li>
    }
}
