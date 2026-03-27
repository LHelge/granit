use chrono::Utc;
use granit_types::{Frontmatter, RenderedNote};
use pulldown_cmark::{html, Options, Parser};

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
/// 2. Resolve `[[wiki-links]]` against `cave_notes`
/// 3. Run the resolved body through pulldown-cmark
pub fn render_note(raw: &str, title: &str, cave_notes: &[&str]) -> RenderedNote {
    let (frontmatter, body) = extract_frontmatter(raw);
    let (resolved_body, outgoing_links) = resolve_wiki_links(body, cave_notes);
    let html = render_html(&resolved_body);
    RenderedNote {
        title: title.to_string(),
        html,
        frontmatter,
        outgoing_links,
    }
}

/// Resolve `[[wiki-links]]` in `body` against the list of cave note slugs.
///
/// For each `[[note-name]]` found:
/// - If a slug matching `note-name` (case-insensitive) exists in `cave_notes`,
///   it is replaced with a standard markdown link `[note-name](slug)`.
/// - If no match is found, it is replaced with an HTML `<span>` carrying the
///   `broken-link` class so the frontend can style it distinctly.
///
/// Returns `(resolved_body, outgoing_links)` where `outgoing_links` is the
/// list of resolved slugs (one entry per resolved wiki-link occurrence).
pub fn resolve_wiki_links(body: &str, cave_notes: &[&str]) -> (String, Vec<String>) {
    let mut output = String::with_capacity(body.len());
    let mut outgoing = Vec::new();
    let mut remaining = body;

    while let Some(open) = remaining.find("[[") {
        // Append everything before the opening `[[`
        output.push_str(&remaining[..open]);
        let after_open = &remaining[open + 2..];

        // Find the matching closing `]]`
        if let Some(close) = after_open.find("]]") {
            let link_text = &after_open[..close];
            // link_text must be non-empty and contain no nested `[` or `]`
            if !link_text.is_empty() && !link_text.contains('[') && !link_text.contains(']') {
                let lower = link_text.to_lowercase();
                if let Some(&slug) = cave_notes.iter().find(|s| s.to_lowercase() == lower) {
                    // Resolved: standard markdown link
                    output.push_str(&format!("[{link_text}]({slug})"));
                    outgoing.push(slug.to_string());
                } else {
                    // Unresolved: broken-link span (raw HTML passthrough in pulldown-cmark)
                    output.push_str(&format!("<span class=\"broken-link\">{link_text}</span>"));
                }
                remaining = &after_open[close + 2..];
                continue;
            }
        }

        // Not a valid wiki-link — emit `[[` literally and advance past it
        output.push_str("[[");
        remaining = after_open;
    }

    output.push_str(remaining);
    (output, outgoing)
}

/// Generate the initial file content for a new note.
///
/// Produces a YAML frontmatter block with `created_at` and `modified_at` set
/// to the current UTC time, followed by a level-1 heading using the slug.
pub fn initial_content(slug: &str) -> String {
    let now = Utc::now();
    let fm = Frontmatter {
        tags: Vec::new(),
        created_at: Some(now),
        modified_at: Some(now),
    };
    let yaml = serde_yml::to_string(&fm).unwrap_or_default();
    format!("---\n{}---\n# {slug}\n", yaml)
}

/// Update the `modified_at` field in the YAML frontmatter of `raw` to the
/// current UTC time.
///
/// If `raw` contains no frontmatter (or it cannot be parsed) the original
/// content is returned unchanged — callers that save user-edited content
/// without frontmatter are not affected.
pub fn update_modified_at(raw: &str) -> String {
    let after_open = match raw.strip_prefix("---") {
        Some(rest) => rest,
        None => return raw.to_string(),
    };
    let after_open = match after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
    {
        Some(rest) => rest,
        None => return raw.to_string(),
    };

    let close_pattern = "\n---";
    let close_pos = match after_open.find(close_pattern) {
        Some(pos) => pos,
        None => return raw.to_string(),
    };

    let yaml = &after_open[..close_pos];
    let after_close = &after_open[close_pos + close_pattern.len()..];

    let mut fm: Frontmatter = match serde_yml::from_str(yaml) {
        Ok(fm) => fm,
        Err(_) => return raw.to_string(),
    };
    fm.modified_at = Some(Utc::now());

    let new_yaml = match serde_yml::to_string(&fm) {
        Ok(s) => s,
        Err(_) => return raw.to_string(),
    };

    format!("---\n{}---{}", new_yaml, after_close)
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
        let result = render_note(raw, "my-note", &[]);
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
        let result = render_note(raw, "plain-note", &[]);
        assert_eq!(result.title, "plain-note");
        assert!(result.frontmatter.is_none());
        assert!(result.html.contains("<h1>"));
    }

    // ── resolve_wiki_links ────────────────────────────────────────────────────

    #[test]
    fn test_wiki_link_no_links() {
        let (out, links) = resolve_wiki_links("Just plain text.", &["some-note"]);
        assert_eq!(out, "Just plain text.");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_resolved() {
        let (out, links) = resolve_wiki_links("See [[my-note]] for details.", &["my-note"]);
        assert_eq!(out, "See [my-note](my-note) for details.");
        assert_eq!(links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_resolved_case_insensitive() {
        let (out, links) = resolve_wiki_links("See [[My-Note]] for details.", &["my-note"]);
        assert_eq!(out, "See [My-Note](my-note) for details.");
        assert_eq!(links, ["my-note"]);
    }

    #[test]
    fn test_wiki_link_unresolved() {
        let (out, links) = resolve_wiki_links("See [[ghost]] here.", &["real-note"]);
        assert!(out.contains("broken-link"), "got: {out}");
        assert!(out.contains("ghost"), "got: {out}");
        assert!(links.is_empty());
    }

    #[test]
    fn test_wiki_link_multiple() {
        let notes = ["alpha", "gamma"];
        let (out, links) = resolve_wiki_links("[[alpha]] and [[beta]] and [[gamma]].", &notes);
        assert!(out.contains("[alpha](alpha)"), "got: {out}");
        assert!(
            out.contains("broken-link") && out.contains("beta"),
            "got: {out}"
        );
        assert!(out.contains("[gamma](gamma)"), "got: {out}");
        assert_eq!(links, ["alpha", "gamma"]);
    }

    #[test]
    fn test_wiki_link_in_render_note_populates_outgoing() {
        let notes = ["other-note"];
        let result = render_note("Check [[other-note]] out.", "my-note", &notes);
        assert_eq!(result.outgoing_links, ["other-note"]);
        assert!(result.html.contains("<a href=\"other-note\">"));
    }

    #[test]
    fn test_wiki_link_empty_brackets_passthrough() {
        let (out, links) = resolve_wiki_links("[[]] is not a link.", &[]);
        assert_eq!(out, "[[]] is not a link.");
        assert!(links.is_empty());
    }
}
