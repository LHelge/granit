use chrono::{Local, Utc};
use granit_types::{Frontmatter, RenderedNote};
use pulldown_cmark::{html, Event, LinkType, Options, Parser, Tag, TagEnd};

/// Render a full markdown document to a [`RenderedNote`].
///
/// `title` is the note's slug (filename without `.md`) — displayed as a page
/// header outside the rendered body area.
///
/// `cave_notes` is the list of all note slugs in the cave (filename without
/// `.md`), used to resolve `[[wiki-links]]`.
///
/// Pipeline:
/// 1. Strip and parse YAML frontmatter (between `---` fences)
/// 2. Render body through pulldown-cmark with `ENABLE_WIKILINKS`, resolving
///    wiki-links during event processing against `cave_notes`
pub fn render_note<'lookup>(
    raw: &str,
    title: &str,
    lookup: impl Fn(&str) -> Option<&'lookup str>,
) -> RenderedNote {
    let (frontmatter, body) = extract_frontmatter(raw);
    let (html, outgoing_links) = render_with_wiki_links(body, lookup);
    let fmt = |d: chrono::DateTime<Utc>| {
        d.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    };
    let created_display = frontmatter.as_ref().and_then(|f| f.created_at).map(fmt);
    let modified_display = frontmatter.as_ref().and_then(|f| f.modified_at).map(fmt);
    RenderedNote {
        title: title.to_string(),
        html,
        frontmatter,
        outgoing_links,
        created_display,
        modified_display,
    }
}

/// Render markdown to HTML, resolving `[[wiki-links]]` during event processing.
///
/// Uses pulldown-cmark's `ENABLE_WIKILINKS` so wiki-links inside code blocks
/// and inline code are naturally ignored by the parser.
///
/// For each wiki-link:
/// - If the lookup resolves the target, the link href is set to the slug.
/// - If unresolved, the link gets a `class="broken-link"` so the frontend can
///   style it distinctly.
///
/// Returns `(html, outgoing_links)`.
fn render_with_wiki_links<'lookup>(
    markdown: &str,
    lookup: impl Fn(&str) -> Option<&'lookup str>,
) -> (String, Vec<String>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_WIKILINKS);

    let parser = Parser::new_ext(markdown, options);
    let mut outgoing_links = Vec::new();

    let events = parser.flat_map(|event| match event {
        // Resolve wiki-links against the cave
        Event::Start(Tag::Link {
            link_type: LinkType::WikiLink { .. },
            dest_url,
            title: _,
            id,
        }) => {
            if let Some(slug) = lookup(&dest_url) {
                outgoing_links.push(slug.to_string());
                vec![Event::Start(Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: slug.to_string().into(),
                    title: "".into(),
                    id,
                })]
            } else {
                vec![Event::Start(Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url,
                    title: "broken-link".into(),
                    id,
                })]
            }
        }
        // Sanitize: convert raw HTML events to escaped text so untrusted
        // content cannot inject scripts or arbitrary markup.
        Event::Html(raw) | Event::InlineHtml(raw) => {
            vec![
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented)),
                Event::Text(raw),
                Event::End(TagEnd::CodeBlock),
            ]
        }
        other => vec![other],
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, events);
    // pulldown-cmark renders link titles as title="…" attributes; convert
    // our broken-link marker to a class so the frontend can style and detect it.
    let html_output = html_output.replace(" title=\"broken-link\"", " class=\"broken-link\"");
    (html_output, outgoing_links)
}

/// Generate the initial file content for a new note.
///
/// Produces a YAML frontmatter block with `created_at` and `modified_at` set
/// to the current UTC time, followed by a level-1 heading using the slug.
pub fn initial_content(_slug: &str) -> String {
    let now = Utc::now();
    let fm = Frontmatter {
        tags: Vec::new(),
        created_at: Some(now),
        modified_at: Some(now),
    };
    let yaml = serde_yml::to_string(&fm).unwrap_or_default();
    format!("---\n{yaml}---\n")
}

/// Update the `modified_at` field in the YAML frontmatter of `raw` to the
/// current UTC time.
/// Return only the body of a note, stripping any YAML frontmatter.
pub fn strip_frontmatter(raw: &str) -> &str {
    let (_fm, body) = extract_frontmatter(raw);
    body
}

/// Read the existing frontmatter from `existing_raw`, update `modified_at`,
/// optionally override tags, and prepend it to `new_body`.
///
/// If the existing content has no parseable frontmatter the body is returned
/// unchanged.
pub fn rebuild_with_frontmatter(
    existing_raw: &str,
    new_body: &str,
    tags: Option<Vec<String>>,
) -> String {
    let (fm, _) = extract_frontmatter(existing_raw);
    let Some(mut fm) = fm else {
        return new_body.to_string();
    };
    fm.modified_at = Some(Utc::now());
    if let Some(tags) = tags {
        fm.tags = tags;
    }
    // Strip any frontmatter the caller may have included in new_body to
    // avoid duplication (e.g. agent tools sending full-file content).
    let (_, body) = extract_frontmatter(new_body);
    let yaml = serde_yml::to_string(&fm).unwrap_or_default();
    format!("---\n{yaml}---\n{body}")
}

/// Strip YAML frontmatter from `raw`, returning `(Option<Frontmatter>, body)`.
///
/// Frontmatter is a block between two `---` lines at the very start of the
/// document. If absent or malformed, `None` is returned and the full input is
/// treated as the body.
fn extract_frontmatter(raw: &str) -> (Option<Frontmatter>, &str) {
    // Must start with "---" (optionally followed by whitespace/newline)
    let after_open = match raw.strip_prefix("---") {
        Some(rest) => rest,
        None => return (None, raw),
    };

    // The opening fence must be immediately followed by a newline (or EOL)
    let after_open = match after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
    {
        Some(rest) => rest,
        None => return (None, raw),
    };

    // Find the closing "---" fence
    let close_pattern = "\n---";
    let close_pos = match after_open.find(close_pattern) {
        Some(pos) => pos,
        None => return (None, raw),
    };

    let yaml = &after_open[..close_pos];
    let after_close = &after_open[close_pos + close_pattern.len()..];
    // Skip optional newline after closing fence
    let body = after_close
        .strip_prefix('\n')
        .or_else(|| after_close.strip_prefix("\r\n"))
        .unwrap_or(after_close);

    let frontmatter = serde_yml::from_str::<Frontmatter>(yaml).ok();
    (frontmatter, body)
}

/// Render a markdown string to HTML using pulldown-cmark.
///
/// Options enabled: tables, strikethrough, task lists, footnotes.
///
/// Raw HTML in the markdown source (`Event::Html`, `Event::InlineHtml`) is
/// escaped to prevent XSS — LLM responses or user content cannot inject
/// arbitrary HTML/JS into the webview.
pub(crate) fn render_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);

    // Sanitize: convert raw HTML events to escaped text so untrusted
    // content cannot inject scripts or arbitrary markup.
    let sanitized = parser.flat_map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => {
            vec![
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented)),
                Event::Text(raw),
                Event::End(TagEnd::CodeBlock),
            ]
        }
        other => vec![other],
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, sanitized);
    html_output
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── render_html (core markdown rendering) ────────────────────────────────

    #[test]
    fn test_heading() {
        let html = render_html("# Hello");
        assert!(html.contains("<h1>Hello</h1>"), "got: {html}");
    }

    #[test]
    fn test_bold_italic() {
        let html = render_html("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_unordered_list() {
        let html = render_html("- one\n- two\n- three");
        assert!(html.contains("<ul>") || html.contains("<li>"));
    }

    #[test]
    fn test_ordered_list() {
        let html = render_html("1. first\n2. second");
        assert!(html.contains("<ol>") || html.contains("<li>"));
    }

    #[test]
    fn test_code_block() {
        let html = render_html("```rust\nfn main() {}\n```");
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_inline_code() {
        let html = render_html("use `foo` here");
        assert!(html.contains("<code>foo</code>"));
    }

    #[test]
    fn test_link() {
        let html = render_html("[Granit](https://example.com)");
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains("Granit"));
    }

    #[test]
    fn test_table() {
        let html = render_html("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
    }

    #[test]
    fn test_strikethrough() {
        let html = render_html("~~gone~~");
        assert!(html.contains("<del>gone</del>"));
    }

    #[test]
    fn test_task_list() {
        let html = render_html("- [x] done\n- [ ] todo");
        assert!(html.contains(r#"type="checkbox""#));
    }

    #[test]
    fn test_empty_string() {
        let html = render_html("");
        assert!(html.is_empty());
    }

    // ── extract_frontmatter ───────────────────────────────────────────────────

    #[test]
    fn test_frontmatter_present() {
        let raw = "---\ntags:\n  - rust\n  - markdown\ncreated_at: \"2026-03-27T10:00:00Z\"\nmodified_at: \"2026-03-27T12:00:00Z\"\n---\n# Body";
        let (fm, body) = extract_frontmatter(raw);
        let fm = fm.expect("frontmatter should be parsed");
        assert_eq!(fm.tags, ["rust", "markdown"]);
        assert!(fm.created_at.is_some());
        assert!(fm.modified_at.is_some());
        assert_eq!(body, "# Body");
    }

    #[test]
    fn test_frontmatter_absent() {
        let raw = "# Just a heading\n\nSome content.";
        let (fm, body) = extract_frontmatter(raw);
        assert!(fm.is_none());
        assert_eq!(body, raw);
    }

    #[test]
    fn test_frontmatter_partial_fields() {
        let raw = "---\ntags:\n  - notes\n---\nBody text";
        let (fm, body) = extract_frontmatter(raw);
        let fm = fm.expect("frontmatter should be parsed");
        assert_eq!(fm.tags, ["notes"]);
        assert!(fm.created_at.is_none());
        assert!(fm.modified_at.is_none());
        assert_eq!(body, "Body text");
    }

    #[test]
    fn test_frontmatter_malformed_yaml() {
        // Invalid YAML parses to None, body is still returned
        let raw = "---\n: invalid: yaml: :\n---\nBody";
        let (fm, body) = extract_frontmatter(raw);
        assert!(fm.is_none());
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_frontmatter_only() {
        let raw = "---\ntags:\n  - empty\n---\n";
        let (fm, body) = extract_frontmatter(raw);
        assert!(fm.is_some());
        assert_eq!(body, "");
    }

    #[test]
    fn test_frontmatter_not_at_start() {
        let raw = "Some text\n---\ntags:\n  - late\n---\nMore text";
        let (fm, body) = extract_frontmatter(raw);
        assert!(fm.is_none());
        assert_eq!(body, raw);
    }

    // ── render_note (full pipeline) ───────────────────────────────────────────

    #[test]
    fn test_render_note_with_frontmatter() {
        let raw = "---\ntags:\n  - rust\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n# Hello";
        let result = render_note(raw, "my-note", |_| None);
        assert_eq!(result.title, "my-note");
        let fm = result
            .frontmatter
            .as_ref()
            .expect("frontmatter should parse");
        assert_eq!(fm.tags, ["rust"]);
        assert!(fm.created_at.is_some());
        assert!(result.html.contains("<h1>"));
        assert!(
            !result.html.contains("tags:"),
            "frontmatter must not appear in HTML"
        );
    }

    #[test]
    fn test_render_note_without_frontmatter() {
        let raw = "# Plain note";
        let result = render_note(raw, "plain-note", |_| None);
        assert_eq!(result.title, "plain-note");
        assert!(result.frontmatter.is_none());
        assert!(result.html.contains("<h1>"));
    }

    // ── wiki-link resolution (via render_with_wiki_links) ──────────────────────

    #[test]
    fn test_wiki_link_no_links() {
        let (html, links) = render_with_wiki_links("Just plain text.", |_| None);
        assert!(html.contains("Just plain text."), "got: {html}");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_resolved() {
        let (html, links) = render_with_wiki_links("See [[my-note]] for details.", |s| {
            ["my-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(
            html.contains("<a href=\"my-note\">my-note</a>"),
            "got: {html}"
        );
        assert_eq!(links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_resolved_case_insensitive() {
        let (html, links) = render_with_wiki_links("See [[My-Note]] for details.", |s| {
            ["my-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(html.contains("href=\"my-note\""), "got: {html}");
        assert!(html.contains("My-Note"), "got: {html}");
        assert_eq!(links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_unresolved() {
        let (html, links) = render_with_wiki_links("See [[ghost]] here.", |s| {
            ["real-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(html.contains("broken-link"), "got: {html}");
        assert!(html.contains("ghost"), "got: {html}");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_multiple() {
        let notes = ["alpha", "gamma"];
        let (html, links) = render_with_wiki_links("[[alpha]] and [[beta]] and [[gamma]].", |s| {
            notes.iter().copied().find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(html.contains("<a href=\"alpha\">alpha</a>"), "got: {html}");
        assert!(
            html.contains("broken-link") && html.contains("beta"),
            "got: {html}"
        );
        assert!(html.contains("<a href=\"gamma\">gamma</a>"), "got: {html}");
        assert_eq!(links, ["alpha", "gamma"]);
    }

    #[test]
    fn test_wiki_link_in_render_note_populates_outgoing() {
        let notes = ["other-note"];
        let result = render_note("Check [[other-note]] out.", "my-note", |s| {
            notes.iter().copied().find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert_eq!(result.outgoing_links, ["other-note"]);
        assert!(result.html.contains("<a href=\"other-note\">"));
    }

    #[test]
    fn test_wiki_link_empty_brackets_passthrough() {
        let (html, links) = render_with_wiki_links("[[]] is not a link.", |_| None);
        // pulldown-cmark treats [[]] as literal text
        assert!(html.contains("[[]]"), "got: {html}");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_in_fenced_code_block() {
        let input = "before\n```\n[[not-a-link]]\n```\nafter [[real]]";
        let (html, links) = render_with_wiki_links(input, |s| {
            ["not-a-link", "real"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        // The fenced block should not contain a resolved link
        assert!(!html.contains("<a href=\"not-a-link\">"), "got: {html}");
        // The normal link should be resolved
        assert!(html.contains("<a href=\"real\">"), "got: {html}");
        assert_eq!(links, ["real"]);
    }

    #[test]
    fn test_wiki_link_in_tilde_code_block() {
        let input = "~~~\n[[not-a-link]]\n~~~\n[[yes]]";
        let (html, links) = render_with_wiki_links(input, |s| {
            ["not-a-link", "yes"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(!html.contains("<a href=\"not-a-link\">"), "got: {html}");
        assert!(html.contains("<a href=\"yes\">"), "got: {html}");
        assert_eq!(links, ["yes"]);
    }

    #[test]
    fn test_wiki_link_in_inline_code() {
        let input = "See `[[not-a-link]]` and [[real]]";
        let (html, links) = render_with_wiki_links(input, |s| {
            ["not-a-link", "real"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(!html.contains("<a href=\"not-a-link\">"), "got: {html}");
        assert!(html.contains("<a href=\"real\">"), "got: {html}");
        assert_eq!(links, ["real"]);
    }

    #[test]
    fn test_wiki_link_in_double_backtick_inline_code() {
        let input = "See ``[[not-a-link]]`` here";
        let (html, links) = render_with_wiki_links(input, |_| Some("not-a-link"));
        assert!(!html.contains("<a href=\"not-a-link\">"), "got: {html}");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_piped() {
        let (html, links) = render_with_wiki_links("See [[target|display text]] here.", |s| {
            ["target"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        assert!(html.contains("href=\"target\""), "got: {html}");
        assert!(html.contains("display text"), "got: {html}");
        assert_eq!(links, ["target"]);
    }

    #[test]
    fn test_wiki_link_in_fenced_code_full_pipeline() {
        let input = "```\n[[skip-me]]\n```\n\n[[resolve-me]]";
        let result = render_note(input, "test", |s| {
            ["resolve-me"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
        });
        // The code block should render [[skip-me]] as text, not a link
        assert!(
            !result.html.contains("<a href=\"skip-me\">"),
            "got: {}",
            result.html
        );
        // The normal link should resolve
        assert!(
            result.html.contains("<a href=\"resolve-me\">"),
            "got: {}",
            result.html
        );
        assert_eq!(result.outgoing_links, ["resolve-me"]);
    }

    // ── HTML sanitization ─────────────────────────────────────────────────────

    #[test]
    fn test_raw_html_script_is_escaped() {
        let html = render_html("<script>alert('xss')</script>");
        assert!(
            !html.contains("<script>"),
            "raw <script> must not pass through: {html}"
        );
    }

    #[test]
    fn test_inline_html_is_escaped() {
        let html = render_html("Hello <b>bold</b> world");
        assert!(
            !html.contains("<b>bold</b>"),
            "inline HTML tags must not pass through: {html}"
        );
    }

    #[test]
    fn test_img_onerror_is_escaped() {
        let html = render_html("<img src=x onerror=alert(1)>");
        assert!(
            !html.contains("<img"),
            "raw <img> tags must not pass through: {html}"
        );
    }

    #[test]
    fn test_raw_html_content_is_visible() {
        let html = render_html("<div>some content</div>");
        assert!(
            html.contains("some content"),
            "text inside raw HTML should still be visible: {html}"
        );
    }

    #[test]
    fn test_markdown_formatting_still_works() {
        let html = render_html("**bold** and `code`");
        assert!(html.contains("<strong>bold</strong>"), "got: {html}");
        assert!(html.contains("<code>code</code>"), "got: {html}");
    }

    #[test]
    fn test_broken_wiki_link_renders_without_raw_html() {
        let result = render_note("See [[nonexistent]].", "test", |_| None);
        assert!(
            result.html.contains("nonexistent"),
            "broken link text should be visible: {}",
            result.html
        );
        assert!(
            result.html.contains("broken-link"),
            "broken-link title should be in the rendered link: {}",
            result.html
        );
    }
}
