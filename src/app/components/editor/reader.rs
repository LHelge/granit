use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::ipc;

use super::use_editor_ctx;

/// Rendered preview of the active note with wiki-link navigation.
#[component]
pub(super) fn Reader() -> impl IntoView {
    let ctx = use_editor_ctx();

    // Intercept clicks on links in rendered markdown.
    // - External links (http/https) open in the system browser.
    // - Wiki-links navigate within the app.
    let on_click = move |ev: leptos::ev::MouseEvent| {
        let Some(target) = ev.target() else { return };
        let anchor = target
            .dyn_ref::<web_sys::Element>()
            .and_then(|el| {
                if el.tag_name().eq_ignore_ascii_case("a") {
                    Some(el.clone())
                } else {
                    el.closest("a").ok().flatten()
                }
            })
            .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok());

        let Some(anchor) = anchor else { return };
        let href = anchor.get_attribute("href").unwrap_or_default();

        if href.is_empty() || href.starts_with('#') || href.starts_with('/') {
            return;
        }

        // External links → open in system browser
        if href.starts_with("http://") || href.starts_with("https://") {
            ev.prevent_default();
            let url = href.clone();
            leptos::task::spawn_local(async move {
                let _ = ipc::open_url(&url).await;
            });
            return;
        }

        ev.prevent_default();
        // Decode percent-encoded characters (e.g. %20 → space) before lookup
        let slug = js_sys::decode_uri_component(&href)
            .ok()
            .and_then(|s| s.as_string())
            .unwrap_or(href);
        let is_broken = anchor.class_list().contains("broken-link");
        ctx.navigate_wiki_link(slug, is_broken);
    };

    view! {
        <h1 class="!mt-0 !mb-1">
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
                                <span class="inline-flex px-2 py-0.5 rounded-full text-xs bg-stone-700 text-stone-300">
                                    {tag}
                                </span>
                            }).collect_view()}
                        </div>
                    })}
                    {created.map(|ts| view! {
                        <span class="text-xs italic text-stone-500">{format!("Created: {ts}")}</span>
                    })}
                    {modified.map(|ts| view! {
                        <span class="text-xs italic text-stone-500">{format!("Modified: {ts}")}</span>
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
