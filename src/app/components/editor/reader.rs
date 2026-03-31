use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::use_editor_ctx;

/// Rendered preview of the active note with wiki-link navigation.
#[component]
pub(super) fn Reader() -> impl IntoView {
    let ctx = use_editor_ctx();

    // Intercept clicks on rendered wiki-links and navigate to the target note.
    // External links (http/https) and anchors (#) pass through normally.
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

        if href.is_empty()
            || href.starts_with("http")
            || href.starts_with('#')
            || href.starts_with('/')
        {
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
            let items: Vec<(&str, String)> = [
                note.created_display.as_deref().map(|s| ("Created", s.to_string())),
                note.modified_display.as_deref().map(|s| ("Edited", s.to_string())),
            ]
            .into_iter()
            .flatten()
            .collect();
            if items.is_empty() {
                return None;
            }
            Some(view! {
                <ul class="not-prose list-none p-0 !mt-0 !mb-6">
                    {items.into_iter().map(|(label, ts)| view! {
                        <li class="text-sm italic text-stone-500">
                            {format!("{label}: {ts}")}
                        </li>
                    }).collect_view()}
                </ul>
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
