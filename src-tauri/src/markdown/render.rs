use chrono::{Local, Utc};
use granit_types::RenderedDocument;
use pulldown_cmark::{html, Event, LinkType, Options, Parser, Tag, TagEnd};

use super::Markdown;

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
    pub fn render<'lookup>(
        &self,
        title: &str,
        lookup: impl Fn(&str) -> Option<&'lookup str>,
    ) -> RenderedDocument {
        let frontmatter = self.frontmatter().cloned();
        let to_owned = |s: &str| lookup(s).map(ToString::to_string);
        let (html, outgoing_links) = render_core(self.body(), Some(&to_owned), true);
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
    pub fn render_with_links<'lookup>(
        &self,
        lookup: impl Fn(&str) -> Option<&'lookup str>,
    ) -> String {
        let to_owned = |s: &str| lookup(s).map(ToString::to_string);
        render_core(self.body(), Some(&to_owned), false).0
    }

    /// Collect resolved outgoing wiki-link slugs without full rendering.
    pub fn outgoing_links<'lookup>(
        &self,
        lookup: impl Fn(&str) -> Option<&'lookup str>,
    ) -> Vec<String> {
        collect_resolved_wiki_links(self.body(), lookup)
    }
}

// ── Shared helpers ───────────────────────────────────────────────────────────

/// Base pulldown-cmark options shared by all render paths.
fn base_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
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

    let events = parser.flat_map(|event| match event {
        // Resolve wiki-links against the cave
        Event::Start(Tag::Link {
            link_type: LinkType::WikiLink { .. },
            dest_url,
            title: _,
            id,
        }) => {
            let lookup = lookup.expect("ENABLE_WIKILINKS set only when lookup is Some");
            if let Some(slug) = lookup(&dest_url) {
                outgoing_links.push(slug.clone());
                vec![Event::Start(Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: slug.into(),
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
            vec![Event::InlineHtml(match (interactive_checkboxes, checked) {
                (true, true) => {
                    r#"<input type="checkbox" class="checkbox checkbox-sm" checked>"#.into()
                }
                (true, false) => r#"<input type="checkbox" class="checkbox checkbox-sm">"#.into(),
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

fn collect_resolved_wiki_links<'lookup>(
    markdown: &str,
    lookup: impl Fn(&str) -> Option<&'lookup str>,
) -> Vec<String> {
    let mut options = base_options();
    options.insert(Options::ENABLE_WIKILINKS);

    Parser::new_ext(markdown, options)
        .filter_map(|event| match event {
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => lookup(&dest_url).map(ToString::to_string),
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

#[cfg(test)]
mod tests {
    use crate::markdown::Markdown;

    // ── render_html (core markdown rendering) ────────────────────────────────

    #[test]
    fn test_heading() {
        let html = Markdown::new("# Hello").render_html();
        assert!(html.contains("<h1>Hello</h1>"), "got: {html}");
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
                "Target" => Some("target"),
                "Other" => Some("other"),
                _ => None,
            });
        assert_eq!(links, vec!["target".to_string(), "other".to_string()]);
    }

    #[test]
    fn test_outgoing_links_skips_broken_targets() {
        let links = Markdown::new("[[Target]] [[Missing]]").outgoing_links(|name| match name {
            "Target" => Some("target"),
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
        });
        assert!(note.html.contains("broken-link"), "got: {}", note.html);
        assert!(note.html.contains("ghost"), "got: {}", note.html);
        assert!(note.outgoing_links.is_empty());
    }

    #[test]
    fn test_wiki_link_multiple() {
        let notes = ["alpha", "gamma"];
        let note = Markdown::new("[[alpha]] and [[beta]] and [[gamma]].").render("t", |s| {
            notes.iter().copied().find(|&k| k.eq_ignore_ascii_case(s))
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
            notes.iter().copied().find(|&k| k.eq_ignore_ascii_case(s))
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
        let note = Markdown::new(input).render("t", |_| Some("not-a-link"));
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
}
