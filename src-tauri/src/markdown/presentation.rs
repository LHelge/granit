//! Presentation rendering: a note's markdown split into slides and
//! assembled into a standalone HTML page for the presentation window.
//!
//! # DOM contract
//!
//! The page is a complete HTML document. Its `<head>` carries the built-in
//! base stylesheet (mechanics only: active-slide toggle, canvas scaling,
//! overflow clipping, cursor hiding) followed by a `<link>` to the cave's
//! template CSS, so the template always wins the cascade. The `<body>` holds
//! one `<section class="slide" data-index="n" data-number="n+1">` per slide
//! containing reader-compatible rendered markdown; exactly one section has
//! the `active` class. Custom properties carry the rest, so templates can use
//! them in `calc()` and `content:` — on `<body>`: `--slide-count` (number),
//! `--deck-title`, `--deck-created`, `--deck-modified` (strings; the note's
//! slug and its frontmatter dates as `YYYY-MM-DD`, empty when absent); on
//! each section: `--slide-index` and `--slide-number` (numbers),
//! `--slide-heading` (the slide's first heading) and `--slide-chapter` (the
//! latest level-1 heading up to and including the slide; strings, empty when
//! there is none). Slides are laid out on a
//! fixed logical canvas of [`SLIDE_WIDTH`] × [`SLIDE_HEIGHT`] CSS pixels,
//! exposed to templates as the `--slide-width` / `--slide-height` custom
//! properties, and scaled to fit the viewport. Content that does not fit a
//! slide is clipped.
//!
//! # Splitting
//!
//! Slides are separated by thematic breaks (`---`, `***`, `___`), detected
//! on the pulldown-cmark event stream so a `---` inside a fenced code block
//! or a table never splits. Empty slides are dropped; a note without a
//! separator is a one-slide presentation. Note that CommonMark reads a
//! `---` directly under a line of text as a setext heading underline, so a
//! separator needs a blank line before it.
//!
//! # Content rules
//!
//! Wiki-links flatten to their label text, task checkboxes render disabled,
//! `{#id}` heading anchors are stripped while the `{.class}` attributes of a
//! slide's first heading move onto the `<section>` as layout classes (any
//! name except the reserved `slide` and `active`), relative image sources
//! point at
//! the cave file route, HTML comments are dropped (so they can hold speaker
//! notes) and any other raw HTML is escaped.

use pulldown_cmark::{html, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};

use super::render::{base_options, rewrite_image_src, sanitize_html_event_vec};
use super::Markdown;

/// Logical slide width in CSS pixels.
#[cfg(test)]
pub const SLIDE_WIDTH: u32 = 1280;
/// Logical slide height in CSS pixels.
#[cfg(test)]
pub const SLIDE_HEIGHT: u32 = 720;

/// Built-in mechanics stylesheet, placed before the template CSS.
const BASE_CSS: &str = include_str!("presentation.css");
/// Page script: canvas scaling, cursor hiding and navigation.
const PAGE_SCRIPT: &str = include_str!("presentation.js");

/// One rendered slide plus the metadata exposed to templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    /// Reader-compatible HTML of the slide's content.
    pub html: String,
    /// Plain text of the slide's first heading, if it has one.
    pub heading: Option<String>,
    /// Plain text of the latest level-1 heading up to and including this
    /// slide: the "chapter" a slide belongs to.
    pub chapter: Option<String>,
    /// Layout classes from the first heading's `{.class}` attributes.
    pub classes: Vec<String>,
}

// Consumed by the presentation window's page route.
#[allow(dead_code)]
impl Markdown<'_> {
    /// Split the body into slides and render each to HTML.
    ///
    /// Returns one entry per non-empty slide, in document order.
    pub fn render_slides(&self) -> Vec<Slide> {
        let mut options = base_options();
        options.insert(Options::ENABLE_WIKILINKS);
        let image_base = self.image_base.as_deref();

        let mut slides = Vec::new();
        let mut chapter: Option<String> = None;
        let mut current: Vec<Event> = Vec::new();
        for event in Parser::new_ext(self.body(), options) {
            if matches!(event, Event::Rule) {
                let events = std::mem::take(&mut current);
                push_slide(&mut slides, events, image_base, &mut chapter);
            } else {
                current.push(event);
            }
        }
        push_slide(&mut slides, current, image_base, &mut chapter);
        slides
    }

    /// Assemble the standalone presentation page.
    ///
    /// `title` becomes the document title and `template_css_href` the URL of
    /// the template stylesheet linked after the base stylesheet.
    pub fn render_presentation(&self, title: &str, template_css_href: &str) -> String {
        let slides = self.render_slides();
        let mut page = String::new();
        page.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
        page.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
        page.push_str(&format!("<title>{}</title>\n", escape_html(title)));
        page.push_str(&format!("<style id=\"granit-base\">\n{BASE_CSS}</style>\n"));
        page.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n",
            escape_html(template_css_href)
        ));
        let date = |d: Option<chrono::DateTime<chrono::Utc>>| {
            d.map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .unwrap_or_default()
        };
        let frontmatter = self.frontmatter();
        let body_style = format!(
            "--slide-count: {}; --deck-title: {}; --deck-created: {}; --deck-modified: {}",
            slides.len(),
            css_string(title),
            css_string(&date(frontmatter.and_then(|f| f.created_at))),
            css_string(&date(frontmatter.and_then(|f| f.modified_at))),
        );
        page.push_str(&format!(
            "</head>\n<body style=\"{}\">\n",
            escape_html(&body_style)
        ));
        for (index, slide) in slides.iter().enumerate() {
            let active = if index == 0 { " active" } else { "" };
            let extra = slide
                .classes
                .iter()
                .map(|c| format!(" {c}"))
                .collect::<String>();
            let style = format!(
                "--slide-index: {index}; --slide-number: {}; --slide-heading: {}; --slide-chapter: {}",
                index + 1,
                css_string(slide.heading.as_deref().unwrap_or_default()),
                css_string(slide.chapter.as_deref().unwrap_or_default()),
            );
            page.push_str(&format!(
                "<section class=\"slide{active}{extra}\" data-index=\"{index}\" data-number=\"{}\" style=\"{}\">\n{}</section>\n",
                index + 1,
                escape_html(&style),
                slide.html
            ));
        }
        page.push_str(&format!(
            "<script>\n{PAGE_SCRIPT}</script>\n</body>\n</html>\n"
        ));
        page
    }
}

/// Render one slide's events and append it unless it is empty, tracking the
/// running chapter (latest level-1 heading) across slides.
fn push_slide(
    slides: &mut Vec<Slide>,
    events: Vec<Event>,
    image_base: Option<&str>,
    chapter: &mut Option<String>,
) {
    let headings = heading_texts(&events);
    let classes = slide_classes(&events);
    let html = render_slide(events, image_base);
    if html.trim().is_empty() {
        return;
    }
    if let Some((_, text)) = headings
        .iter()
        .rev()
        .find(|(level, _)| *level == HeadingLevel::H1)
    {
        *chapter = Some(text.clone());
    }
    slides.push(Slide {
        html,
        heading: headings.into_iter().next().map(|(_, text)| text),
        chapter: chapter.clone(),
        classes,
    });
}

/// Class names reserved by the base stylesheet and the page script.
const RESERVED_CLASSES: [&str; 2] = ["slide", "active"];

/// Layout classes for a slide: the `{.class}` attributes of its first
/// heading, keeping only well-formed CSS identifiers that are not reserved.
fn slide_classes(events: &[Event]) -> Vec<String> {
    let first_heading = events.iter().find_map(|event| match event {
        Event::Start(Tag::Heading { classes, .. }) => Some(classes),
        _ => None,
    });
    let mut out: Vec<String> = Vec::new();
    for class in first_heading.into_iter().flatten() {
        let class = class.trim();
        let valid = class
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
            && class
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if valid && !RESERVED_CLASSES.contains(&class) && !out.iter().any(|c| c == class) {
            out.push(class.to_string());
        }
    }
    out
}

/// The plain text of every heading in a slide, in document order.
fn heading_texts(events: &[Event]) -> Vec<(HeadingLevel, String)> {
    let mut headings = Vec::new();
    let mut open: Option<(HeadingLevel, String)> = None;
    for event in events {
        match event {
            Event::Start(Tag::Heading { level, .. }) => open = Some((*level, String::new())),
            Event::End(TagEnd::Heading(_)) => headings.extend(open.take()),
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, buf)) = open.as_mut() {
                    buf.push_str(text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, buf)) = open.as_mut() {
                    buf.push(' ');
                }
            }
            _ => {}
        }
    }
    headings
        .into_iter()
        .map(|(level, text)| (level, text.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect()
}

/// Quote `text` as a single-quoted CSS string, escaping backslashes, quotes
/// and line breaks, so it is safe inside a `style` attribute value.
fn css_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Render one slide's event stream, applying the presentation content rules.
fn render_slide(events: Vec<Event>, image_base: Option<&str>) -> String {
    let mut in_wiki_link = false;
    let events = events.into_iter().flat_map(|event| match event {
        // Wiki-links flatten to their label: drop the link tags, keep the text.
        Event::Start(Tag::Link {
            link_type: LinkType::WikiLink { .. },
            ..
        }) => {
            in_wiki_link = true;
            vec![]
        }
        Event::End(TagEnd::Link) if in_wiki_link => {
            in_wiki_link = false;
            vec![]
        }
        // Headings render plain: the anchor id is a note-level concern and
        // the classes have moved onto the section (see `slide_classes`).
        Event::Start(Tag::Heading { level, .. }) => vec![Event::Start(Tag::Heading {
            level,
            id: None,
            classes: Vec::new(),
            attrs: Vec::new(),
        })],
        // Checkboxes are never interactive on a slide.
        Event::TaskListMarker(checked) => vec![Event::InlineHtml(
            if checked {
                r#"<input type="checkbox" class="checkbox checkbox-sm" checked disabled>"#
            } else {
                r#"<input type="checkbox" class="checkbox checkbox-sm" disabled>"#
            }
            .into(),
        )],
        // Point relative image sources at the cave file route.
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let dest_url = image_base
                .and_then(|base| rewrite_image_src(base, &dest_url))
                .map_or(dest_url, Into::into);
            vec![Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            })]
        }
        // HTML comments are speaker notes: dropped. Other raw HTML is escaped.
        Event::Html(raw) | Event::InlineHtml(raw) => {
            if is_html_comment(&raw) {
                vec![]
            } else {
                sanitize_html_event_vec(raw)
            }
        }
        other => vec![other],
    });

    let mut html = String::new();
    html::push_html(&mut html, events);
    html
}

/// Whether a raw HTML event consists only of comments (and whitespace).
fn is_html_comment(raw: &str) -> bool {
    let mut rest = raw.trim();
    if rest.is_empty() {
        return false;
    }
    while let Some(after_open) = rest.strip_prefix("<!--") {
        match after_open.find("-->") {
            Some(end) => rest = after_open[end + 3..].trim_start(),
            None => return false,
        }
    }
    rest.is_empty()
}

/// Escape text for use in HTML content or a double-quoted attribute.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Splitting ─────────────────────────────────────────────────────

    #[test]
    fn test_all_thematic_break_variants_split() {
        let slides = Markdown::new("# One\n\n---\n\n# Two\n\n***\n\n# Three\n\n___\n\n# Four\n")
            .render_slides();
        assert_eq!(slides.len(), 4, "got: {slides:?}");
        assert!(slides[0].html.contains("<h1>One</h1>"));
        assert!(slides[3].html.contains("<h1>Four</h1>"));
    }

    #[test]
    fn test_separator_inside_code_block_does_not_split() {
        let md = "# One\n\n```\n---\n***\n```\n\n~~~\n___\n~~~\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides.len(), 1, "got: {slides:?}");
        assert!(slides[0].html.contains("---"), "got: {}", slides[0].html);
    }

    #[test]
    fn test_table_separator_row_does_not_split() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides.len(), 1, "got: {slides:?}");
        assert!(slides[0].html.contains("<table>"));
    }

    #[test]
    fn test_empty_slides_are_dropped() {
        let md = "---\n\n# One\n\n---\n\n---\n\n# Two\n\n---\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides.len(), 2, "got: {slides:?}");
        assert!(slides[0].html.contains("One"));
        assert!(slides[1].html.contains("Two"));
    }

    #[test]
    fn test_no_separator_is_one_slide_and_no_title_slide_is_added() {
        let slides = Markdown::new("Just a paragraph.").render_slides();
        assert_eq!(slides.len(), 1);
        assert!(slides[0].html.contains("<p>Just a paragraph.</p>"));
    }

    #[test]
    fn test_empty_body_has_no_slides() {
        assert!(Markdown::new("").render_slides().is_empty());
        assert!(Markdown::new("---\n\n---\n").render_slides().is_empty());
    }

    #[test]
    fn test_frontmatter_is_stripped() {
        let md = "---\ntags: [x]\npresentation: dark\n---\n# Title\n\n---\n\nBody\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides.len(), 2, "got: {slides:?}");
        assert!(
            !slides[0].html.contains("presentation:"),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_separator_directly_under_text_is_a_setext_heading() {
        // CommonMark: `---` right under a paragraph underlines it as an h2,
        // so it does not split. Documented behaviour, not a bug.
        let slides = Markdown::new("Text\n---\nMore\n").render_slides();
        assert_eq!(slides.len(), 1);
        assert!(
            slides[0].html.contains("<h2>Text</h2>"),
            "got: {}",
            slides[0].html
        );
    }

    // ── Content rules ─────────────────────────────────────────────────

    #[test]
    fn test_wiki_links_flatten_to_label_text() {
        let slides = Markdown::new("See [[Other Note]] and [[note|the label]].").render_slides();
        assert!(
            slides[0].html.contains("See Other Note and the label."),
            "got: {}",
            slides[0].html
        );
        assert!(!slides[0].html.contains("<a "), "got: {}", slides[0].html);
    }

    #[test]
    fn test_regular_links_are_kept() {
        let slides = Markdown::new("[ext](https://example.com)").render_slides();
        assert!(
            slides[0]
                .html
                .contains(r#"<a href="https://example.com">ext</a>"#),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_checkboxes_render_disabled() {
        let slides = Markdown::new("- [ ] open\n- [x] done\n").render_slides();
        assert_eq!(
            slides[0].html.matches("disabled").count(),
            2,
            "got: {}",
            slides[0].html
        );
        assert!(
            slides[0].html.contains("checked disabled"),
            "got: {}",
            slides[0].html
        );
        assert!(
            !slides[0].html.contains("data-index"),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_heading_anchor_attributes_are_stripped() {
        let slides = Markdown::new("# Volvo {#volvo .title key=val}").render_slides();
        assert!(
            slides[0].html.contains("<h1>Volvo</h1>"),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_first_heading_classes_become_slide_classes() {
        let md = "# Intro {.section .dark}\n\n---\n\ntext\n\n## Later {.first}\n\n### Sub {.ignored}\n\n---\n\n## Two {.two-column .slide .active .1bad .ok_1 .two-column}\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides[0].classes, vec!["section", "dark"]);
        // Only the first heading of a slide describes the slide.
        assert_eq!(slides[1].classes, vec!["first"]);
        // Reserved and malformed names are dropped; duplicates collapse.
        assert_eq!(slides[2].classes, vec!["two-column", "ok_1"]);

        let page = Markdown::new(md).render_presentation("t", "t.css");
        assert!(
            page.contains(r#"<section class="slide active section dark" data-index="0""#),
            "got: {page}"
        );
        assert!(
            page.contains(r#"<section class="slide two-column ok_1" data-index="2""#),
            "got: {page}"
        );
        // The heading itself stays plain.
        assert!(page.contains("<h1>Intro</h1>"), "got: {page}");
    }

    #[test]
    fn test_code_block_keeps_reader_language_class() {
        let slides = Markdown::new("```rust\nfn main() {}\n```").render_slides();
        assert!(
            slides[0]
                .html
                .contains(r#"<pre><code class="language-rust">"#),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_relative_images_use_cave_route() {
        let slides = Markdown::new("![](img/a.png)")
            .with_image_base("talks")
            .render_slides();
        let expected = crate::scheme::cave_file_url("talks/img/a.png");
        assert!(
            slides[0].html.contains(&expected),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_html_comments_are_dropped_and_other_html_escaped() {
        let md = "# Slide\n\n<!-- speaker notes: breathe -->\n\n<script>alert(1)</script>\n";
        let slides = Markdown::new(md).render_slides();
        assert_eq!(slides.len(), 1);
        assert!(
            !slides[0].html.contains("speaker notes"),
            "got: {}",
            slides[0].html
        );
        assert!(
            !slides[0].html.contains("<script>"),
            "got: {}",
            slides[0].html
        );
        assert!(
            slides[0].html.contains("&lt;script&gt;"),
            "got: {}",
            slides[0].html
        );
    }

    #[test]
    fn test_slide_with_only_a_comment_is_empty() {
        let slides =
            Markdown::new("# One\n\n---\n\n<!-- notes only -->\n\n---\n\n# Two\n").render_slides();
        assert_eq!(slides.len(), 2, "got: {slides:?}");
    }

    #[test]
    fn test_is_html_comment() {
        assert!(is_html_comment("<!-- a -->"));
        assert!(is_html_comment("<!-- a -->\n<!-- b -->\n"));
        assert!(!is_html_comment("<!-- a --><b>x</b>"));
        assert!(!is_html_comment("<!-- unterminated"));
        assert!(!is_html_comment("<div>"));
        assert!(!is_html_comment(""));
    }

    // ── Page assembly (DOM contract) ──────────────────────────────────

    #[test]
    fn test_page_dom_contract() {
        let page = Markdown::new("# One\n\n---\n\n# Two\n\n---\n\n# Three\n")
            .render_presentation("My <Talk>", "granit://localhost/presentation/dark.css");

        assert!(page.starts_with("<!DOCTYPE html>"), "got: {page}");
        assert!(
            page.contains("<title>My &lt;Talk&gt;</title>"),
            "got: {page}"
        );

        // Base stylesheet precedes the template link.
        let base_at = page.find("<style id=\"granit-base\">").expect("base css");
        let link_at = page
            .find(r#"<link rel="stylesheet" href="granit://localhost/presentation/dark.css">"#)
            .expect("template link");
        assert!(base_at < link_at);

        // Canvas custom properties are exposed by the base stylesheet.
        assert!(page.contains("--slide-width: 1280px"), "got: {page}");
        assert!(page.contains("--slide-height: 720px"), "got: {page}");
        assert!(page.contains("--slide-scale"), "got: {page}");
        assert!(page.contains("overflow: hidden"), "got: {page}");

        // One section per slide, first active, indexed.
        assert_eq!(page.matches("<section class=\"slide").count(), 3);
        assert_eq!(page.matches("class=\"slide active\"").count(), 1);
        assert!(
            page.contains(
                r#"<section class="slide active" data-index="0" data-number="1" style="--slide-index: 0; --slide-number: 1; --slide-heading: 'One'; --slide-chapter: 'One'">"#
            ),
            "got: {page}"
        );
        assert!(
            page.contains(
                r#"<section class="slide" data-index="2" data-number="3" style="--slide-index: 2; --slide-number: 3; --slide-heading: 'Three'; --slide-chapter: 'Three'">"#
            ),
            "got: {page}"
        );
        assert!(
            page.contains(
                r#"<body style="--slide-count: 3; --deck-title: 'My &lt;Talk&gt;'; --deck-created: ''; --deck-modified: ''">"#
            ),
            "got: {page}"
        );
        // Hidden by an unconditional rule, so templates may set `display` on
        // `.slide` or a layout class without revealing inactive slides.
        assert!(
            page.contains(".slide:not(.active) {\n  display: none !important;"),
            "got: {page}"
        );

        // The script slot comes after the slides.
        let last_section = page.rfind("</section>").unwrap();
        let script_at = page.find("<script>").unwrap();
        assert!(script_at > last_section);
    }

    #[test]
    fn test_slide_heading_and_chapter_metadata() {
        let md = "# Intro\n\n---\n\n## Agenda\n\ntext\n\n---\n\nNo heading here\n\n---\n\n# Part `two`\n\n## Detail\n\n---\n\n## More *detail*\n";
        let slides = Markdown::new(md).render_slides();
        let meta: Vec<(Option<&str>, Option<&str>)> = slides
            .iter()
            .map(|s| (s.heading.as_deref(), s.chapter.as_deref()))
            .collect();
        assert_eq!(
            meta,
            vec![
                (Some("Intro"), Some("Intro")),
                (Some("Agenda"), Some("Intro")),
                (None, Some("Intro")),
                (Some("Part two"), Some("Part two")),
                (Some("More detail"), Some("Part two")),
            ]
        );

        // No headings at all: both stay empty.
        let slides = Markdown::new("just text").render_slides();
        assert_eq!(slides[0].heading, None);
        assert_eq!(slides[0].chapter, None);
    }

    #[test]
    fn test_page_exposes_dates_and_escapes_strings() {
        let md = "---\ncreated_at: \"2026-09-01T10:00:00Z\"\nmodified_at: \"2026-09-09T10:00:00Z\"\n---\n# It's \"quoted\" \\ back\n";
        let page = Markdown::new(md).render_presentation("Bob's talk", "t.css");
        assert!(
            page.contains(r#"--deck-title: 'Bob\'s talk'"#),
            "got: {page}"
        );
        assert!(page.contains("--deck-created: '2026-09-01'"), "got: {page}");
        assert!(
            page.contains("--deck-modified: '2026-09-09'"),
            "got: {page}"
        );
        assert!(
            page.contains(r#"--slide-heading: 'It\'s &quot;quoted&quot; \\ back'"#),
            "got: {page}"
        );
    }

    #[test]
    fn test_css_string() {
        assert_eq!(css_string(""), "''");
        assert_eq!(css_string("plain"), "'plain'");
        assert_eq!(css_string("a'b\\c\nd"), "'a\\'b\\\\c d'");
    }

    #[test]
    fn test_page_escapes_template_href() {
        let page = Markdown::new("x").render_presentation("t", "a\"b<c");
        assert!(page.contains(r#"href="a&quot;b&lt;c""#), "got: {page}");
    }

    #[test]
    fn test_canvas_constants_match_base_css() {
        assert!(BASE_CSS.contains(&format!("--slide-width: {SLIDE_WIDTH}px")));
        assert!(BASE_CSS.contains(&format!("--slide-height: {SLIDE_HEIGHT}px")));
    }
}
