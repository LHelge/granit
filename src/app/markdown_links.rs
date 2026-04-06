use wasm_bindgen::JsCast;

pub(crate) enum MarkdownLinkTarget {
    External(String),
    Wiki { slug: String, is_broken: bool },
}

pub(crate) fn classify_markdown_link_target(
    target: Option<web_sys::EventTarget>,
) -> Option<MarkdownLinkTarget> {
    let anchor = target
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| {
            if el.tag_name().eq_ignore_ascii_case("a") {
                Some(el)
            } else {
                el.closest("a").ok().flatten()
            }
        })
        .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok())?;

    let href = anchor.get_attribute("href").unwrap_or_default();
    if href.is_empty() || href.starts_with('#') || href.starts_with('/') {
        return None;
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(MarkdownLinkTarget::External(href));
    }

    let slug = js_sys::decode_uri_component(&href)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or(href);
    let is_broken = anchor.class_list().contains("broken-link");

    Some(MarkdownLinkTarget::Wiki { slug, is_broken })
}
