use chrono::{Local, Utc};
use granit_types::RenderedDocument;
use pulldown_cmark::{html, BlockQuoteKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use std::ops::Range;

use super::Markdown;

impl Markdown<'_> {
    /// Parse a markdown string and return only the plain text content,
    /// stripping all formatting, links, images, etc.
    pub fn strip(md: &str) -> String {
        let mut opts = base_options();
        opts.insert(Options::ENABLE_WIKILINKS);
        let parser = Parser::new_ext(md, opts);
        let mut plain = String::new();
        for event in parser {
            if let Event::Text(text) | Event::Code(text) = event {
                plain.push_str(&text);
            }
        }
        plain
    }
}

impl Markdown<'_> {
    /// Source line numbers (1-based, counted in the full raw document
    /// including frontmatter) of each rendered task-list checkbox, in
    /// render order.
    ///
    /// The Nth entry is the line of the checkbox the renderer tagged
    /// `data-index="N"`, because both walk the same pulldown-cmark event
    /// stream. The reader's toggle-by-index path must map indexes to lines
    /// through this — counting raw `- [ ]` lines instead also counts text
    /// inside fenced code blocks (and misses ordered-list and blockquote
    /// checkboxes), silently toggling the wrong line.
    pub fn checkbox_source_lines(&self) -> Vec<usize> {
        let mut opts = base_options();
        opts.insert(Options::ENABLE_WIKILINKS);
        let body = self.body();
        // `body` is a tail slice of the raw document.
        let body_offset = self.raw.len() - body.len();
        Parser::new_ext(body, opts)
            .into_offset_iter()
            .filter_map(|(event, range)| {
                matches!(event, Event::TaskListMarker(_))
                    .then(|| self.raw[..body_offset + range.start].matches('\n').count() + 1)
            })
            .collect()
    }
}

impl<'a> Markdown<'a> {
    /// Render this markdown document to a [`RenderedDocument`].
    ///
    /// `title` is the note's slug (filename without `.md`).
    /// `lookup` resolves wiki-link targets to canonical slugs.
    ///
    /// Pipeline:
    /// 1. Strip and parse YAML frontmatter (between `---` fences)
    /// 2. Render body through pulldown-cmark with `ENABLE_WIKILINKS`, resolving
    ///    wiki-links during event processing against `lookup`
    pub fn render(
        &self,
        title: &str,
        resolve: impl Fn(&str) -> Option<String>,
    ) -> RenderedDocument {
        let frontmatter = self.frontmatter().cloned();
        let (html, outgoing_links) = render_core(self.body(), Some(&resolve), true);
        let fmt = |d: chrono::DateTime<Utc>| {
            d.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        };
        let created_display = frontmatter.as_ref().and_then(|f| f.created_at).map(fmt);
        let modified_display = frontmatter.as_ref().and_then(|f| f.modified_at).map(fmt);
        RenderedDocument {
            title: title.to_string(),
            html,
            frontmatter,
            outgoing_links,
            backlinks: Vec::new(),
            created_display,
            modified_display,
        }
    }

    /// Render markdown to standalone HTML (no wiki-link resolution).
    pub fn render_html(&self) -> String {
        render_core(self.body(), None, false).0
    }

    /// Render markdown to HTML with wiki-link resolution.
    ///
    /// Used for agent chat messages where wiki-links should be clickable
    /// but checkboxes are non-interactive.
    pub fn render_with_links(&self, resolve: impl Fn(&str) -> Option<String>) -> String {
        render_core(self.body(), Some(&resolve), false).0
    }

    /// Collect resolved outgoing wiki-link **note** slugs without full rendering.
    ///
    /// `resolve` returns the link href (`note` or `note#anchor`); the fragment is
    /// stripped so the returned slugs are always note-level (for the backlink graph).
    pub fn outgoing_links(&self, resolve: impl Fn(&str) -> Option<String>) -> Vec<String> {
        collect_resolved_wiki_links(self.body(), resolve)
    }

    /// Collect the anchor ids declared by `{#id}` heading attributes in the body.
    ///
    /// These are the headings that act as wiki-link targets. Plain headings
    /// without an attribute carry no id and are not returned.
    pub fn anchor_ids(&self) -> Vec<String> {
        let options = base_options();
        Parser::new_ext(self.body(), options)
            .filter_map(|event| match event {
                Event::Start(Tag::Heading { id: Some(id), .. }) => Some(id.to_string()),
                _ => None,
            })
            .collect()
    }
}

impl Markdown<'_> {
    /// Rewrite every wiki-link whose target resolves (case-insensitively) to
    /// `old_slug` so it points to `new_slug`, preserving any `|label`. Returns
    /// `Some(rewritten)` if at least one link changed, else `None`.
    ///
    /// Operates on raw bytes via pulldown-cmark offsets, so links inside code
    /// spans/blocks (which are not real wiki-links) are left untouched. The
    /// caller passes the full file content; frontmatter is irrelevant since it
    /// never contains wiki-links, and splicing raw bytes preserves it verbatim.
    pub fn rename_wiki_links(text: &str, old_slug: &str, new_slug: &str) -> Option<String> {
        let mut opts = base_options();
        opts.insert(Options::ENABLE_WIKILINKS);

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for (event, range) in Parser::new_ext(text, opts).into_offset_iter() {
            if let Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) = event
            {
                if dest_url.trim().eq_ignore_ascii_case(old_slug) {
                    if let Some(replacement) = rewrite_wiki_span(&text[range.clone()], new_slug) {
                        edits.push((range, replacement));
                    }
                }
            }
        }

        if edits.is_empty() {
            return None;
        }

        let mut out = text.to_string();
        // Apply right-to-left so earlier byte ranges stay valid.
        for (range, replacement) in edits.into_iter().rev() {
            out.replace_range(range, &replacement);
        }
        Some(out)
    }
}

/// Replace the target of a raw wiki-link span (`[[target]]` / `[[target|label]]`)
/// with `new_slug`, preserving the brackets and any display label.
fn rewrite_wiki_span(span: &str, new_slug: &str) -> Option<String> {
    let inner = span.strip_prefix("[[")?.strip_suffix("]]")?;
    Some(match inner.split_once('|') {
        Some((_target, label)) => format!("[[{new_slug}|{label}]]"),
        None => format!("[[{new_slug}]]"),
    })
}

// ── Shared helpers ───────────────────────────────────────────────────────────

/// Base pulldown-cmark options shared by all render paths.
fn base_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_GFM);
    // Parse pandoc-style `# Heading {#id}` attributes. The `{#id}` both renders
    // as `<h1 id="id">` (the scroll anchor) and marks the heading as a wiki-link
    // target; plain headings without an attribute are not link targets.
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options
}

/// Lookup function that resolves a wiki-link target name to an owned slug.
type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Core render pipeline used by all three public methods.
///
/// - `lookup`: when `Some`, enables `ENABLE_WIKILINKS` and resolves links via
///   the closure. When `None`, wiki-link syntax is left as-is (literal text).
/// - `interactive_checkboxes`: if `true`, emits checkboxes without `disabled`.
///
/// Returns `(html, outgoing_links)`.
fn render_core(
    markdown: &str,
    lookup: Option<Lookup>,
    interactive_checkboxes: bool,
) -> (String, Vec<String>) {
    let mut options = base_options();
    if lookup.is_some() {
        options.insert(Options::ENABLE_WIKILINKS);
    }

    let parser = Parser::new_ext(markdown, options);
    let mut outgoing_links = Vec::new();
    let mut in_broken_link = false;
    let mut alert_kind: Option<BlockQuoteKind> = None;
    // Stable per-checkbox index emitted as `data-index` so the frontend can
    // identify the toggled todo without relying on DOM-order counting
    // (which breaks when the renderer emits disabled or nested checkboxes).
    let mut checkbox_index: usize = 0;

    let events = parser.flat_map(|event| match event {
        // ── Blockquote alerts ────────────────────────────────────────
        Event::Start(Tag::BlockQuote(Some(kind))) => {
            alert_kind = Some(kind);
            vec![Event::InlineHtml(alert_open_html(kind).into())]
        }
        Event::End(TagEnd::BlockQuote(_)) if alert_kind.is_some() => {
            alert_kind = None;
            vec![Event::InlineHtml("</div>".into())]
        }
        // Resolve wiki-links against the cave
        Event::Start(Tag::Link {
            link_type: LinkType::WikiLink { .. },
            dest_url,
            title: _,
            id,
        }) => {
            let lookup = lookup.expect("ENABLE_WIKILINKS set only when lookup is Some");
            if let Some(href) = lookup(&dest_url) {
                // The href may carry a `#anchor` fragment; the backlink graph is
                // note-level, so record only the note slug.
                let note_slug = href.split_once('#').map_or(href.as_str(), |(n, _)| n);
                outgoing_links.push(note_slug.to_string());
                vec![Event::Start(Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: href.clone().into(),
                    title: "".into(),
                    id,
                })]
            } else {
                in_broken_link = true;
                let escaped = dest_url.replace('&', "&amp;").replace('"', "&quot;");
                vec![Event::InlineHtml(
                    format!(r#"<a href="{escaped}" class="broken-link">"#).into(),
                )]
            }
        }
        // Close the manually-opened broken-link <a> tag.
        Event::End(TagEnd::Link) if in_broken_link => {
            in_broken_link = false;
            vec![Event::InlineHtml("</a>".into())]
        }
        // Emit styled checkboxes; interactive in the note reader, disabled in
        // agent responses where toggling todos is not supported.
        Event::TaskListMarker(checked) => {
            let idx = checkbox_index;
            checkbox_index += 1;
            vec![Event::InlineHtml(match (interactive_checkboxes, checked) {
                (true, true) => format!(
                    r#"<input type="checkbox" class="checkbox checkbox-sm" data-index="{idx}" checked>"#
                )
                .into(),
                (true, false) => format!(
                    r#"<input type="checkbox" class="checkbox checkbox-sm" data-index="{idx}">"#
                )
                .into(),
                (false, true) => {
                    r#"<input type="checkbox" class="checkbox checkbox-sm" checked disabled>"#
                        .into()
                }
                (false, false) => {
                    r#"<input type="checkbox" class="checkbox checkbox-sm" disabled>"#.into()
                }
            })]
        }
        // Sanitize raw HTML
        Event::Html(raw) | Event::InlineHtml(raw) => sanitize_html_event_vec(raw),
        other => vec![other],
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, events);
    (html_output, outgoing_links)
}

fn collect_resolved_wiki_links(
    markdown: &str,
    resolve: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut options = base_options();
    options.insert(Options::ENABLE_WIKILINKS);

    Parser::new_ext(markdown, options)
        .filter_map(|event| match event {
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => resolve(&dest_url).map(|href| {
                // Strip any `#anchor` fragment: backlinks are note-level.
                href.split_once('#')
                    .map_or(href.clone(), |(note, _)| note.to_string())
            }),
            _ => None,
        })
        .collect()
}

/// Convert raw HTML into escaped code block events so untrusted content
/// cannot inject scripts or arbitrary markup.
fn sanitize_html_event_vec(raw: pulldown_cmark::CowStr) -> Vec<Event> {
    vec![
        Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented)),
        Event::Text(raw),
        Event::End(TagEnd::CodeBlock),
    ]
}

/// Emit the opening HTML for a GitHub-style blockquote alert.
fn alert_open_html(kind: BlockQuoteKind) -> String {
    let (class, label, icon) = alert_meta(kind);
    format!(
        r#"<div class="markdown-alert {class}"><p class="markdown-alert-title">{icon} {label}</p>"#,
    )
}

/// Return (CSS class, display label, inline SVG icon) for each alert type.
fn alert_meta(kind: BlockQuoteKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BlockQuoteKind::Note => (
            "markdown-alert-note",
            "Note",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>"#,
        ),
        BlockQuoteKind::Tip => (
            "markdown-alert-tip",
            "Tip",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"/><path d="M9 18h6"/><path d="M10 22h4"/></svg>"#,
        ),
        BlockQuoteKind::Important => (
            "markdown-alert-important",
            "Important",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#,
        ),
        BlockQuoteKind::Warning => (
            "markdown-alert-warning",
            "Warning",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#,
        ),
        BlockQuoteKind::Caution => (
            "markdown-alert-caution",
            "Caution",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="7.86 2 16.14 2 22 7.86 22 16.14 16.14 22 7.86 22 2 16.14 2 7.86 7.86 2"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>"#,
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::markdown::Markdown;

    // ── render_html (core markdown rendering) ────────────────────────────────

    #[test]
    fn test_heading() {
        let html = Markdown::new("# Hello").render_html();
        assert!(html.contains("<h1>Hello</h1>"), "got: {html}");
    }

    // ── Heading anchors ───────────────────────────────────────────────

    #[test]
    fn test_marked_heading_renders_id() {
        let html = Markdown::new("# Volvo {#volvo}").render_html();
        assert!(html.contains(r#"<h1 id="volvo">Volvo</h1>"#), "got: {html}");
    }

    #[test]
    fn test_plain_heading_has_no_id_and_no_anchor() {
        let md = Markdown::new("# Volvo");
        let html = md.render_html();
        assert!(html.contains("<h1>Volvo</h1>"), "got: {html}");
        assert!(md.anchor_ids().is_empty());
    }

    #[test]
    fn test_anchor_ids_collects_marked_headings_only() {
        let md = Markdown::new("# Volvo {#volvo}\n\nbody\n\n## SAAB\n\n### Ford {#ford}");
        assert_eq!(
            md.anchor_ids(),
            vec!["volvo".to_string(), "ford".to_string()]
        );
    }

    #[test]
    fn test_wiki_link_to_anchor_resolves_with_fragment() {
        // A resolver that maps the anchor name to a `note#anchor` href.
        let note = Markdown::new("See [[Volvo]].").render("t", |s| {
            (s.eq_ignore_ascii_case("volvo")).then(|| "car-brands#volvo".to_string())
        });
        assert!(
            note.html
                .contains(r#"<a href="car-brands#volvo">Volvo</a>"#),
            "got: {}",
            note.html
        );
        // The backlink graph stays note-level: the fragment is stripped.
        assert_eq!(note.outgoing_links, ["car-brands"]);
    }

    #[test]
    fn test_outgoing_links_strips_anchor_fragment() {
        let links = Markdown::new("[[Volvo]] and [[plain]]").outgoing_links(|name| match name {
            "Volvo" => Some("car-brands#volvo".to_string()),
            "plain" => Some("plain".to_string()),
            _ => None,
        });
        assert_eq!(links, vec!["car-brands".to_string(), "plain".to_string()]);
    }

    #[test]
    fn test_bold_italic() {
        let html = Markdown::new("**bold** and *italic*").render_html();
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_unordered_list() {
        let html = Markdown::new("- one\n- two\n- three").render_html();
        assert!(html.contains("<ul>") || html.contains("<li>"));
    }

    #[test]
    fn test_ordered_list() {
        let html = Markdown::new("1. first\n2. second").render_html();
        assert!(html.contains("<ol>") || html.contains("<li>"));
    }

    #[test]
    fn test_code_block() {
        let html = Markdown::new("```rust\nfn main() {}\n```").render_html();
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_inline_code() {
        let html = Markdown::new("use `foo` here").render_html();
        assert!(html.contains("<code>foo</code>"));
    }

    #[test]
    fn test_link() {
        let html = Markdown::new("[Granit](https://example.com)").render_html();
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains("Granit"));
    }

    #[test]
    fn test_table() {
        let html = Markdown::new("| A | B |\n|---|---|\n| 1 | 2 |").render_html();
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
    }

    #[test]
    fn test_strikethrough() {
        let html = Markdown::new("~~gone~~").render_html();
        assert!(html.contains("<del>gone</del>"));
    }

    #[test]
    fn test_task_list() {
        let note = Markdown::new("- [x] done\n- [ ] todo").render("test", |_| None);
        assert!(!note.html.contains("disabled"), "got: {}", note.html);
        assert!(
            note.html.contains(r#"class="checkbox checkbox-sm""#),
            "got: {}",
            note.html
        );
        assert!(note.html.contains("checked"), "got: {}", note.html);
    }

    #[test]
    fn test_empty_string() {
        let html = Markdown::new("").render_html();
        assert!(html.is_empty());
    }

    // ── outgoing_links ───────────────────────────────────────────────

    #[test]
    fn test_outgoing_links_collects_resolved_targets() {
        let links =
            Markdown::new("[[Target]] and [[Other|label]]").outgoing_links(|name| match name {
                "Target" => Some("target".to_string()),
                "Other" => Some("other".to_string()),
                _ => None,
            });
        assert_eq!(links, vec!["target".to_string(), "other".to_string()]);
    }

    #[test]
    fn test_outgoing_links_skips_broken_targets() {
        let links = Markdown::new("[[Target]] [[Missing]]").outgoing_links(|name| match name {
            "Target" => Some("target".to_string()),
            _ => None,
        });
        assert_eq!(links, vec!["target".to_string()]);
    }

    // ── wiki-link resolution ──────────────────────────────────────────

    #[test]
    fn test_wiki_link_no_links() {
        let note = Markdown::new("Just plain text.").render("t", |_| None);
        assert!(note.html.contains("Just plain text."), "got: {}", note.html);
        assert!(note.outgoing_links.is_empty());
    }

    #[test]
    fn test_wiki_link_resolved() {
        let note = Markdown::new("See [[my-note]] for details.").render("t", |s| {
            ["my-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            note.html.contains("<a href=\"my-note\">my-note</a>"),
            "got: {}",
            note.html
        );
        assert_eq!(note.outgoing_links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_resolved_case_insensitive() {
        let note = Markdown::new("See [[My-Note]] for details.").render("t", |s| {
            ["my-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(note.html.contains("href=\"my-note\""), "got: {}", note.html);
        assert!(note.html.contains("My-Note"), "got: {}", note.html);
        assert_eq!(note.outgoing_links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_unresolved() {
        let note = Markdown::new("See [[ghost]] here.").render("t", |s| {
            ["real-note"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(note.html.contains("broken-link"), "got: {}", note.html);
        assert!(note.html.contains("ghost"), "got: {}", note.html);
        assert!(note.outgoing_links.is_empty());
    }

    #[test]
    fn test_wiki_link_multiple() {
        let notes = ["alpha", "gamma"];
        let note = Markdown::new("[[alpha]] and [[beta]] and [[gamma]].").render("t", |s| {
            notes
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            note.html.contains("<a href=\"alpha\">alpha</a>"),
            "got: {}",
            note.html
        );
        assert!(
            note.html.contains("broken-link") && note.html.contains("beta"),
            "got: {}",
            note.html
        );
        assert!(
            note.html.contains("<a href=\"gamma\">gamma</a>"),
            "got: {}",
            note.html
        );
        assert_eq!(note.outgoing_links, ["alpha", "gamma"]);
    }

    #[test]
    fn test_wiki_link_in_render_populates_outgoing() {
        let notes = ["other-note"];
        let result = Markdown::new("Check [[other-note]] out.").render("my-note", |s| {
            notes
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert_eq!(result.outgoing_links, ["other-note"]);
        assert!(result.html.contains("<a href=\"other-note\">"));
    }

    #[test]
    fn test_wiki_link_empty_brackets_passthrough() {
        let note = Markdown::new("[[]] is not a link.").render("t", |_| None);
        assert!(note.html.contains("[[]]"), "got: {}", note.html);
        assert!(note.outgoing_links.is_empty());
    }

    #[test]
    fn test_wiki_link_in_fenced_code_block() {
        let input = "before\n```\n[[not-a-link]]\n```\nafter [[real]]";
        let note = Markdown::new(input).render("t", |s| {
            ["not-a-link", "real"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            !note.html.contains("<a href=\"not-a-link\">"),
            "got: {}",
            note.html
        );
        assert!(
            note.html.contains("<a href=\"real\">"),
            "got: {}",
            note.html
        );
        assert_eq!(note.outgoing_links, ["real"]);
    }

    #[test]
    fn test_wiki_link_in_tilde_code_block() {
        let input = "~~~\n[[not-a-link]]\n~~~\n[[yes]]";
        let note = Markdown::new(input).render("t", |s| {
            ["not-a-link", "yes"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            !note.html.contains("<a href=\"not-a-link\">"),
            "got: {}",
            note.html
        );
        assert!(note.html.contains("<a href=\"yes\">"), "got: {}", note.html);
        assert_eq!(note.outgoing_links, ["yes"]);
    }

    #[test]
    fn test_wiki_link_in_inline_code() {
        let input = "See `[[not-a-link]]` and [[real]]";
        let note = Markdown::new(input).render("t", |s| {
            ["not-a-link", "real"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            !note.html.contains("<a href=\"not-a-link\">"),
            "got: {}",
            note.html
        );
        assert!(
            note.html.contains("<a href=\"real\">"),
            "got: {}",
            note.html
        );
        assert_eq!(note.outgoing_links, ["real"]);
    }

    #[test]
    fn test_wiki_link_in_double_backtick_inline_code() {
        let input = "See ``[[not-a-link]]`` here";
        let note = Markdown::new(input).render("t", |_| Some("not-a-link".to_string()));
        assert!(
            !note.html.contains("<a href=\"not-a-link\">"),
            "got: {}",
            note.html
        );
        assert!(note.outgoing_links.is_empty());
    }

    #[test]
    fn test_wiki_link_piped() {
        let note = Markdown::new("See [[target|display text]] here.").render("t", |s| {
            ["target"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(note.html.contains("href=\"target\""), "got: {}", note.html);
        assert!(note.html.contains("display text"), "got: {}", note.html);
        assert_eq!(note.outgoing_links, ["target"]);
    }

    #[test]
    fn test_wiki_link_in_fenced_code_full_pipeline() {
        let input = "```\n[[skip-me]]\n```\n\n[[resolve-me]]";
        let result = Markdown::new(input).render("test", |s| {
            ["resolve-me"]
                .iter()
                .copied()
                .find(|&k| k.eq_ignore_ascii_case(s))
                .map(String::from)
        });
        assert!(
            !result.html.contains("<a href=\"skip-me\">"),
            "got: {}",
            result.html
        );
        assert!(
            result.html.contains("<a href=\"resolve-me\">"),
            "got: {}",
            result.html
        );
        assert_eq!(result.outgoing_links, ["resolve-me"]);
    }

    // ── rename_wiki_links ─────────────────────────────────────────────

    #[test]
    fn test_rename_wiki_links_bare() {
        let out = Markdown::rename_wiki_links("See [[old]] here.", "old", "new").unwrap();
        assert_eq!(out, "See [[new]] here.");
    }

    #[test]
    fn test_rename_wiki_links_preserves_label() {
        let out = Markdown::rename_wiki_links("See [[old|My Label]].", "old", "new").unwrap();
        assert_eq!(out, "See [[new|My Label]].");
    }

    #[test]
    fn test_rename_wiki_links_case_insensitive_target() {
        let out = Markdown::rename_wiki_links("See [[Old]] here.", "old", "new").unwrap();
        assert_eq!(out, "See [[new]] here.");
    }

    #[test]
    fn test_rename_wiki_links_multiple_in_one_note() {
        let out =
            Markdown::rename_wiki_links("[[old]] and [[old|again]] and [[old]]", "old", "new")
                .unwrap();
        assert_eq!(out, "[[new]] and [[new|again]] and [[new]]");
    }

    #[test]
    fn test_rename_wiki_links_only_matched_target() {
        let out = Markdown::rename_wiki_links("[[old]] and [[other]]", "old", "new").unwrap();
        assert_eq!(out, "[[new]] and [[other]]");
    }

    #[test]
    fn test_rename_wiki_links_skips_code_block() {
        let input = "```\n[[old]]\n```\nthen [[old]]";
        let out = Markdown::rename_wiki_links(input, "old", "new").unwrap();
        assert_eq!(out, "```\n[[old]]\n```\nthen [[new]]");
    }

    #[test]
    fn test_rename_wiki_links_skips_inline_code() {
        let input = "`[[old]]` and [[old]]";
        let out = Markdown::rename_wiki_links(input, "old", "new").unwrap();
        assert_eq!(out, "`[[old]]` and [[new]]");
    }

    #[test]
    fn test_rename_wiki_links_no_match_returns_none() {
        assert!(Markdown::rename_wiki_links("See [[other]] here.", "old", "new").is_none());
        assert!(Markdown::rename_wiki_links("No links at all.", "old", "new").is_none());
    }

    #[test]
    fn test_rename_wiki_links_preserves_frontmatter_and_body() {
        let input = "---\ntags:\n- a\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nLink to [[old]].";
        let out = Markdown::rename_wiki_links(input, "old", "new").unwrap();
        assert_eq!(
            out,
            "---\ntags:\n- a\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nLink to [[new]]."
        );
    }

    // ── HTML sanitization ─────────────────────────────────────────────────────

    #[test]
    fn test_raw_html_script_is_escaped() {
        let html = Markdown::new("<script>alert('xss')</script>").render_html();
        assert!(
            !html.contains("<script>"),
            "raw <script> must not pass through: {html}"
        );
    }

    #[test]
    fn test_inline_html_is_escaped() {
        let html = Markdown::new("Hello <b>bold</b> world").render_html();
        assert!(
            !html.contains("<b>bold</b>"),
            "inline HTML tags must not pass through: {html}"
        );
    }

    #[test]
    fn test_img_onerror_is_escaped() {
        let html = Markdown::new("<img src=x onerror=alert(1)>").render_html();
        assert!(
            !html.contains("<img"),
            "raw <img> tags must not pass through: {html}"
        );
    }

    #[test]
    fn test_raw_html_content_is_visible() {
        let html = Markdown::new("<div>some content</div>").render_html();
        assert!(
            html.contains("some content"),
            "text inside raw HTML should still be visible: {html}"
        );
    }

    #[test]
    fn test_markdown_formatting_still_works() {
        let html = Markdown::new("**bold** and `code`").render_html();
        assert!(html.contains("<strong>bold</strong>"), "got: {html}");
        assert!(html.contains("<code>code</code>"), "got: {html}");
    }

    #[test]
    fn test_broken_wiki_link_renders_without_raw_html() {
        let result = Markdown::new("See [[nonexistent]].").render("test", |_| None);
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

    // ── render with frontmatter ───────────────────────────────────────────────

    #[test]
    fn test_render_with_frontmatter() {
        let raw = "---\ntags:\n  - rust\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n# Hello";
        let result = Markdown::new(raw).render("my-note", |_| None);
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
    fn test_render_without_frontmatter() {
        let raw = "# Plain note";
        let result = Markdown::new(raw).render("plain-note", |_| None);
        assert_eq!(result.title, "plain-note");
        assert!(result.frontmatter.is_none());
        assert!(result.html.contains("<h1>"));
    }

    // ── Blockquote alerts ─────────────────────────────────────────────────────

    #[test]
    fn test_alert_note() {
        let html = Markdown::new("> [!NOTE]\n> Some useful info.").render_html();
        assert!(
            html.contains("markdown-alert-note"),
            "should have note class: {html}"
        );
        assert!(html.contains("Note"), "should have Note label: {html}");
        assert!(
            html.contains("Some useful info."),
            "should have content: {html}"
        );
    }

    #[test]
    fn test_alert_tip() {
        let html = Markdown::new("> [!TIP]\n> A helpful tip.").render_html();
        assert!(
            html.contains("markdown-alert-tip"),
            "should have tip class: {html}"
        );
        assert!(html.contains("Tip"), "should have Tip label: {html}");
    }

    #[test]
    fn test_alert_important() {
        let html = Markdown::new("> [!IMPORTANT]\n> Critical info.").render_html();
        assert!(
            html.contains("markdown-alert-important"),
            "should have important class: {html}"
        );
        assert!(
            html.contains("Important"),
            "should have Important label: {html}"
        );
    }

    #[test]
    fn test_alert_warning() {
        let html = Markdown::new("> [!WARNING]\n> Be careful.").render_html();
        assert!(
            html.contains("markdown-alert-warning"),
            "should have warning class: {html}"
        );
        assert!(
            html.contains("Warning"),
            "should have Warning label: {html}"
        );
    }

    #[test]
    fn test_alert_caution() {
        let html = Markdown::new("> [!CAUTION]\n> Dangerous action.").render_html();
        assert!(
            html.contains("markdown-alert-caution"),
            "should have caution class: {html}"
        );
        assert!(
            html.contains("Caution"),
            "should have Caution label: {html}"
        );
    }

    #[test]
    fn test_regular_blockquote_unchanged() {
        let html = Markdown::new("> Just a regular quote.").render_html();
        assert!(
            html.contains("<blockquote>"),
            "regular blockquote should use <blockquote>: {html}"
        );
        assert!(
            !html.contains("markdown-alert"),
            "regular blockquote should not have alert class: {html}"
        );
    }

    #[test]
    fn test_alert_with_nested_content() {
        let input = "> [!NOTE]\n> Some text with **bold** and `code`.\n>\n> - item 1\n> - item 2";
        let html = Markdown::new(input).render_html();
        assert!(
            html.contains("markdown-alert-note"),
            "should have note class: {html}"
        );
        assert!(
            html.contains("<strong>bold</strong>"),
            "should render bold inside alert: {html}"
        );
        assert!(
            html.contains("<code>code</code>"),
            "should render code inside alert: {html}"
        );
    }
}
