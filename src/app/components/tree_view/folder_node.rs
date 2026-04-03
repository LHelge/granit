use leptos::prelude::*;
use web_sys::{DragEvent, MouseEvent};

use super::rename_input::RenameInput;
use super::tree_model::TreeNode;
use super::{render_node, use_tree_ctx};
use super::{ContextMenu, ContextTarget, DragPayload, RenameTarget};
use crate::app::components::icons::Icon;
use crate::app::ipc;
use icondata_lu;

/// Renders a folder row in the tree, with collapsing, drag-drop, context menu,
/// and rename support. Recursively renders children.
#[component]
pub(super) fn FolderNode(
    name: String,
    path: String,
    children: Vec<TreeNode>,
    indent_style: String,
    depth: usize,
) -> impl IntoView {
    let ctx = use_tree_ctx();
    let open = RwSignal::new(true);
    let drag_over = RwSignal::new(false);

    let children_views = children
        .into_iter()
        .map(|child| render_node(child, depth + 1))
        .collect_view();

    let on_dragstart = {
        let path = path.clone();
        move |e: DragEvent| {
            e.stop_propagation();
            if let Some(dt) = e.data_transfer() {
                let _ = dt.set_data("text/plain", &path);
            }
            ctx.drag_payload
                .set(Some(DragPayload::Folder(path.clone())));
        }
    };
    let on_dragover = move |e: DragEvent| {
        if ctx.drag_payload.get().is_some() {
            e.prevent_default();
            e.stop_propagation();
            drag_over.set(true);
        }
    };
    let on_dragleave = move |_| drag_over.set(false);

    view! {
        <li draggable="true" on:dragstart=on_dragstart>
            {move || {
                let is_renaming = matches!(
                    ctx.renaming.get(),
                    Some(RenameTarget::Folder(ref p)) if *p == path
                );

                if is_renaming {
                    let path = path.clone();
                    view! {
                        <RenameInput
                            initial=name.clone()
                            indent_style=indent_style.clone()
                            note=false
                            on_confirm=Callback::new(move |new_name: String| {
                                let source = path.clone();
                                ctx.renaming.set(None);
                                let old_name = source.rsplit('/').next().unwrap_or(&source).to_string();
                                if new_name == old_name || new_name.is_empty() {
                                    return;
                                }
                                leptos::task::spawn_local(async move {
                                    match ipc::rename_folder(&source, &new_name).await {
                                        Ok(()) => ctx.refresh_async().await,
                                        Err(e) => ctx.push_error(format!("Failed to rename folder: {e}")),
                                    }
                                });
                            })
                            on_cancel=Callback::new(move |()| ctx.renaming.set(None))
                        />
                    }
                    .into_any()
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
                                ctx.context_menu.set(Some(ContextMenu {
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
                                if let Some(payload) = ctx.drag_payload.get() {
                                    ctx.drag_payload.set(None);
                                    ctx.handle_drop(payload, Some(path_drop.clone()));
                                }
                            }
                        >
                            <span class=move || if open.get() { "inline-flex w-3 h-3 shrink-0 transition-transform rotate-90" } else { "inline-flex w-3 h-3 shrink-0 transition-transform" }>
                                <Icon icon=icondata_lu::LuChevronRight width="100%" height="100%"/>
                            </span>
                            <Icon icon=icondata_lu::LuFolder width="0.875rem" height="0.875rem"/>
                            {name.clone()}
                        </button>
                    }
                    .into_any()
                }
            }}
            <ul class=move || if open.get() { "" } else { "hidden" }>
                {children_views}
            </ul>
        </li>
    }
}
