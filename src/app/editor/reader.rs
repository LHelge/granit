use super::use_editor_ctx;
use crate::app::{
    components::icons::Icon,
    ipc,
    markdown_links::{classify_markdown_link_target, MarkdownLinkTarget},
    AppCtx,
};
use granit_types::resolve_note_icon;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Rendered preview of the active note with wiki-link navigation.
#[component]
pub(super) fn Reader() -> impl IntoView {
    let ctx = use_editor_ctx();
    let app_ctx = expect_context::<AppCtx>();

    // Intercept clicks on links and checkboxes in rendered markdown.
    // - Checkboxes toggle the underlying markdown via the backend.
    // - External links (http/https) open in the system browser.
    // - Wiki-links navigate within the app.
    let on_click = move |ev: leptos::ev::MouseEvent| {
        let Some(target) = ev.target() else { return };

        // --- Checkbox click: toggle via backend ---
        if let Some(checkbox) = target
            .dyn_ref::<web_sys::Element>()
            .and_then(|el| el.dyn_ref::<web_sys::HtmlInputElement>())
            .filter(|inp| inp.type_() == "checkbox")
        {
            ev.prevent_default();

            // ev.current_target() is the div with on:click — i.e. the rendered
            // markdown container.  Count checkboxes within it to find the index.
            let index = ev
                .current_target()
                .and_then(|ct| ct.dyn_into::<web_sys::Element>().ok())
                .and_then(|container| {
                    container
                        .query_selector_all("input[type='checkbox']")
                        .ok()
                        .and_then(|list| {
                            let cb_node: &web_sys::Node = checkbox.as_ref();
                            let len = list.length();
                            for i in 0..len {
                                if let Some(node) = list.item(i) {
                                    if &node == cb_node {
                                        return Some(i as usize);
                                    }
                                }
                            }
                            None
                        })
                })
                .unwrap_or(0);

            if let Some(slug) = ctx.active_note.get_untracked().map(|n| n.meta.slug.clone()) {
                let ctx_inner = ctx;
                let app = app_ctx;
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::toggle_todo_by_index(&slug, index).await {
                        app.push_error("reader", format!("Failed to toggle todo: {e}"));
                        return;
                    }
                    // Re-render the note so the checkbox state reflects what's on disk
                    match ipc::render_note(&slug).await {
                        Ok(rendered) => ctx_inner.rendered_note.set(Some(rendered)),
                        Err(e) => {
                            app.push_error("reader", format!("Failed to re-render note: {e}"));
                        }
                    }
                });
            }
            return;
        }

        // --- Link click ---
        let Some(link) = classify_markdown_link_target(Some(target)) else {
            return;
        };

        ev.prevent_default();
        match link {
            MarkdownLinkTarget::External(url) => {
                leptos::task::spawn_local(async move {
                    let _ = ipc::open_url(&url).await;
                });
            }
            MarkdownLinkTarget::Wiki { slug, is_broken } => ctx.navigate_wiki_link(slug, is_broken),
        }
    };

    view! {
        <h1 class="!mt-0 !mb-1 flex items-center gap-2">
            {move || ctx.icon.get().map(|id| view! {
                <span class="inline-flex w-6 h-6 shrink-0 text-accent">
                    <Icon icon=resolve_note_icon(&id) width="100%" height="100%"/>
                </span>
            })}
            {move || ctx.favorite.get().unwrap_or(false).then(|| view! {
                <span class="inline-flex w-5 h-5 shrink-0 text-warning" aria-label="Favorite note">
                    <Icon icon=icondata_lu::LuStar width="100%" height="100%"/>
                </span>
            })}
            {move || ctx.rendered_note.get().map(|r| r.title).unwrap_or_default()}
        </h1>
        {move || {
            let note = ctx.rendered_note.get()?;
            let tags = note
                .frontmatter
                .map(|fm| fm.tags)
                .unwrap_or_default();
            let created = note.created_display;
            let modified = note.modified_display;
            if tags.is_empty() && created.is_none() && modified.is_none() {
                return None;
            }
            Some(view! {
                <div class="not-prose !mt-0 !mb-4 flex flex-col gap-0.5">
                    {(!tags.is_empty()).then(|| view! {
                        <div class="flex flex-wrap items-center gap-2 mb-2">
                            {tags.into_iter().map(|tag| view! {
                                <span class="badge badge-ghost badge-sm">
                                    {tag}
                                </span>
                            }).collect_view()}
                        </div>
                    })}
                    {created.map(|ts| view! {
                        <span class="text-xs italic text-base-content/35">{format!("Created: {ts}")}</span>
                    })}
                    {modified.map(|ts| view! {
                        <span class="text-xs italic text-base-content/35">{format!("Modified: {ts}")}</span>
                    })}
                </div>
            })
        }}
        <div
            style:font-family=move || ctx.config.get().reading_font.font_family
            style:font-size=move || format!("{}px", ctx.config.get().reading_font.font_size)
            inner_html=move || ctx.rendered_note.get().map(|r| r.html).unwrap_or_default()
            on:click=on_click
        />
    }
}
