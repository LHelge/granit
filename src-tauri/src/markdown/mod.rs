use granit_types::{Frontmatter, RenderedNote};
use pulldown_cmark::{html, Options, Parser};

/// Render a full markdown document to a [`RenderedNote`].
///
/// `title` is the note's slug (filename without `.md`) — displayed as a page
/// header outside the rendered body area.
///
/// Pipeline:
/// 1. Strip and parse YAML frontmatter (between `---` fences)
/// 2. Run the body through pulldown-cmark
///
/// Wiki-link resolution is handled by the next stage in the pipeline.
pub fn render_note(raw: &str, title: &str) -> RenderedNote {
    let (frontmatter, body) = extract_frontmatter(raw);
    let html = render_html(body);
    RenderedNote {
        title: title.to_string(),
        html,
        frontmatter,
        outgoing_links: Vec::new(),
    }
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
fn render_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
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
        let result = render_note(raw, "my-note");
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
        let result = render_note(raw, "plain-note");
        assert_eq!(result.title, "plain-note");
        assert!(result.frontmatter.is_none());
        assert!(result.html.contains("<h1>"));
    }
}
