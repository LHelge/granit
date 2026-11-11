use super::{
    rename_input::RenameInput, use_tree_ctx, ContextMenu, ContextTarget, DragPayload, RenameTarget,
};
use crate::app::{components::icons::Icon, ipc};
use granit_types::resolve_note_icon;
use granit_types::DocumentMeta;
use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

/// Renders a single note row in the tree, with drag, context menu, and rename support.
#[component]
pub(super) fn NoteNode(meta: DocumentMeta, indent_style: String) -> impl IntoView {
    let ctx = use_tree_ctx();
    let slug = meta.slug.clone();

    let on_dragstart = {
        let slug = slug.clone();
        move |e: DragEvent| {
            e.stop_propagation();
            let slugs = ctx.drag_slugs_for(&slug);
            if let Some(dt) = e.data_transfer() {
                let _ = dt.set_data("text/plain", &slugs.join(","));
            }
            ctx.drag_payload.set(Some(DragPayload::Notes(slugs)));
        }
    };

    view! {
        <li>
            {move || {
                let is_renaming = matches!(
                    ctx.renaming.get(),
                    Some(RenameTarget::Note(ref s)) if *s == slug
                );

                if is_renaming {
                    let slug = slug.clone();
                    view! {
                        <RenameInput
                            initial=slug.clone()
                            indent_style=indent_style.clone()
                            note=true
                            on_confirm=Callback::new(move |new_name: String| {
                                let old = slug.clone();
                                ctx.renaming.set(None);
                                if new_name == old || new_name.is_empty() {
                                    return;
                                }
                                leptos::task::spawn_local(async move {
                                    match ipc::rename_note(&old, &new_name).await {
                                        Ok(new_meta) => {
                                            if ctx.active_note.get().map(|n| n.meta.slug == old).unwrap_or(false) {
                                                if let Ok(note) = ipc::read_note(&new_meta.slug).await {
                                                    ctx.app.set_active_note_document(note);
                                                }
                                            }
                                            ctx.refresh_async().await;
                                        }
                                        Err(e) => ctx.push_error(format!("Failed to rename note: {e}")),
                                    }
                                });
                            })
                            on_cancel=Callback::new(move |()| ctx.renaming.set(None))
                        />
                    }
                    .into_any()
                } else {
                    let slug = slug.clone();
                    let slug_click = slug.clone();
                    let slug_ctx = slug.clone();
                    let slug_row = slug.clone();
                    // The row spans the full width for highlighting, but only
                    // the icon + name is interactive: clicks to the right of
                    // the name fall through to the tree background.
                    view! {
                        <div class=move || {
                            let base = "flex py-0.5 text-sm transition-colors";
                            let is_active = ctx.active_note.get().map(|n| n.meta.slug == slug_row).unwrap_or(false);
                            if is_active {
                                format!("{base} bg-base-content/10 text-base-content")
                            } else if ctx.selected.get().contains(&slug_row) {
                                format!("{base} bg-primary/20 text-base-content")
                            } else {
                                format!("{base} text-base-content/60 has-[>button:hover]:bg-base-content/5 has-[>button:hover]:text-base-content")
                            }
                        }>
                        <button
                            class="min-w-0 max-w-full text-left flex items-center gap-1"
                            style=indent_style.clone()
                            draggable="true"
                            on:dragstart=on_dragstart.clone()
                            on:click=move |e: MouseEvent| {
                                let ctrl = e.ctrl_key() || e.meta_key();
                                let shift = e.shift_key();
                                let load = ctx.select_click(&slug_click, ctrl, shift);
                                if !load {
                                    return;
                                }
                                let s = slug_click.clone();
                                leptos::task::spawn_local(async move {
                                    match ipc::read_note(&s).await {
                                        Ok(note) => ctx.app.set_active_note_document(note),
                                        Err(e) => ctx.push_error(format!("Failed to load note: {e}")),
                                    }
                                });
                            }
                            on:contextmenu=move |e: MouseEvent| {
                                e.prevent_default();
                                e.stop_propagation();
                                ctx.context_menu.set(Some(ContextMenu {
                                    x: e.client_x(),
                                    y: e.client_y(),
                                    target: ContextTarget::Note(slug_ctx.clone()),
                                }));
                            }
                        >
                            <span class="w-3 shrink-0" />
                            <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                <Icon icon=resolve_note_icon(meta.icon.as_deref().unwrap_or("")) width="100%" height="100%"/>
                            </span>
                            <Show when=move || meta.favorite.unwrap_or(false)>
                                <span class="inline-flex w-3 h-3 shrink-0 text-warning">
                                    <Icon icon=icondata_lu::LuStar width="100%" height="100%"/>
                                </span>
                            </Show>
                            <span class="truncate">{slug.clone()}</span>
                        </button>
                        </div>
                    }
                    .into_any()
                }
            }}
        </li>
    }
}
