use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Scroll the element with the given id (a heading anchor) into view.
///
/// Retries across a bounded number of animation frames so navigation works
/// whether the target note was just switched in (DOM not yet rendered) or is
/// already open (element present immediately).
pub(crate) fn scroll_to_anchor(id: String) {
    fn attempt(id: String, remaining: u32) {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Some(el) = window.document().and_then(|d| d.get_element_by_id(&id)) {
            el.scroll_into_view();
            return;
        }
        if remaining == 0 {
            return;
        }
        let cb = Closure::once_into_js(move || attempt(id, remaining - 1));
        let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
    }
    attempt(id, 30);
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MarkdownLinkTarget {
    External(String),
    Wiki {
        slug: String,
        /// Heading anchor id (`note#anchor` wiki links), if present.
        fragment: Option<String>,
        is_broken: bool,
    },
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

    // Split off an optional `#anchor` fragment (heading-anchor wiki links).
    let (path, fragment_raw) = match href.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (href.as_str(), None),
    };

    let decode = |s: &str| {
        js_sys::decode_uri_component(s)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| s.to_string())
    };

    let slug = decode(path);
    let fragment = fragment_raw.map(decode);
    let is_broken = anchor.class_list().contains("broken-link");

    Some(MarkdownLinkTarget::Wiki {
        slug,
        fragment,
        is_broken,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window().unwrap().document().unwrap()
    }

    #[wasm_bindgen_test]
    fn classifies_external_link_from_anchor_target() {
        let anchor = document()
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();
        anchor.set_href("https://example.com/docs");

        let target = classify_markdown_link_target(Some(anchor.into()));

        assert_eq!(
            target,
            Some(MarkdownLinkTarget::External(
                "https://example.com/docs".into()
            ))
        );
    }

    #[wasm_bindgen_test]
    fn classifies_nested_target_inside_wiki_link() {
        let document = document();
        let anchor = document
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();
        anchor.set_attribute("href", "daily%20note").unwrap();

        let child = document.create_element("span").unwrap();
        anchor.append_child(&child).unwrap();

        let target = classify_markdown_link_target(Some(child.into()));

        assert_eq!(
            target,
            Some(MarkdownLinkTarget::Wiki {
                slug: "daily note".into(),
                fragment: None,
                is_broken: false,
            })
        );
    }

    #[wasm_bindgen_test]
    fn parses_heading_anchor_fragment() {
        let anchor = document()
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();
        anchor.set_attribute("href", "car%20brands#volvo").unwrap();

        let target = classify_markdown_link_target(Some(anchor.into()));

        assert_eq!(
            target,
            Some(MarkdownLinkTarget::Wiki {
                slug: "car brands".into(),
                fragment: Some("volvo".into()),
                is_broken: false,
            })
        );
    }

    #[wasm_bindgen_test]
    fn marks_broken_wiki_links_from_class_name() {
        let anchor = document()
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();
        anchor
            .set_attribute("href", "folder%2Fmissing-note")
            .unwrap();
        anchor.set_class_name("broken-link");

        let target = classify_markdown_link_target(Some(anchor.into()));

        assert_eq!(
            target,
            Some(MarkdownLinkTarget::Wiki {
                slug: "folder/missing-note".into(),
                fragment: None,
                is_broken: true,
            })
        );
    }

    #[wasm_bindgen_test]
    fn ignores_empty_fragment_and_root_relative_links() {
        let document = document();
        for href in ["", "#section", "/settings"] {
            let anchor = document
                .create_element("a")
                .unwrap()
                .dyn_into::<web_sys::HtmlAnchorElement>()
                .unwrap();
            anchor.set_attribute("href", href).unwrap();

            let target = classify_markdown_link_target(Some(anchor.into()));
            assert_eq!(target, None, "href {href:?} should be ignored");
        }
    }
}
