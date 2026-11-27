use granit_types::Frontmatter;

use super::Markdown;

impl<'a> Markdown<'a> {
    /// Parsed YAML frontmatter, if present and well-formed.
    pub fn frontmatter(&self) -> Option<&Frontmatter> {
        self.parsed().0.as_ref()
    }

    /// The markdown body with YAML frontmatter stripped.
    pub fn body(&self) -> &'a str {
        self.parsed().1
    }

    /// Shorthand: extract the `icon` field from frontmatter.
    pub fn icon(&self) -> Option<String> {
        self.frontmatter().and_then(|fm| fm.icon.clone())
    }

    /// Shorthand: extract the `favorite` field from frontmatter.
    pub fn favorite(&self) -> Option<bool> {
        self.frontmatter().and_then(|fm| fm.favorite)
    }

    /// Shorthand: extract the `presentation` field from frontmatter.
    pub fn presentation(&self) -> Option<String> {
        self.frontmatter().and_then(|fm| fm.presentation.clone())
    }

    /// Shorthand: extract the `tags` field from frontmatter.
    pub fn tags(&self) -> Vec<String> {
        self.frontmatter()
            .map(|fm| fm.tags.clone())
            .unwrap_or_default()
    }
}

/// The location of a frontmatter block inside a note's raw text.
///
/// The raw text is always `raw[..yaml_start]` (optional BOM plus the opening
/// fence line) + `yaml` + `raw[yaml_end..body_start]` (the closing fence
/// line with its line ending, if any) + the body, so the block can be edited
/// by splicing the YAML text and copying everything else verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrontmatterSpan<'a> {
    /// The YAML lines between the fences, each with its line ending.
    pub yaml: &'a str,
    /// Byte offset where `yaml` starts in the raw text.
    pub yaml_start: usize,
    /// Byte offset where `yaml` ends (the start of the closing fence line).
    pub yaml_end: usize,
    /// Byte offset of the first body byte.
    pub body_start: usize,
    /// The line ending of the opening fence, used for any line Granit adds.
    pub eol: &'static str,
}

const BOM: char = '\u{feff}';

/// Whether `line` (with or without its line ending) is a `---` fence, or a
/// `...` fence when `closing`. Trailing spaces and tabs are allowed.
fn is_fence(line: &str, closing: bool) -> bool {
    let text = line
        .trim_end_matches(['\n', '\r'])
        .trim_end_matches([' ', '\t']);
    text == "---" || (closing && text == "...")
}

/// Find the frontmatter block at the start of `raw`, if there is one.
///
/// The rule that separates frontmatter from a body that merely starts with
/// a `---`-fenced block:
///
/// 1. The file starts (after an optional UTF-8 BOM) with a `---` line, and a
///    later line is `---` or `...`. The first such line closes the block.
/// 2. The text between the fences is frontmatter when it is blank, when it
///    parses as a YAML mapping, or — so malformed frontmatter is kept rather
///    than shown as body text — when it fails to parse as YAML but its first
///    non-blank, non-comment line looks like a `key:` line.
///
/// Anything else (a paragraph between two thematic breaks, a heading between
/// rules, a YAML scalar or list) is body. Only the first block of a file is
/// ever considered: whatever follows the closing fence is body, verbatim, so
/// a note with frontmatter whose body starts with its own
/// `---\nkey: value\n---` keeps that block. A note *without* frontmatter
/// whose body starts with such a block is indistinguishable from one with
/// frontmatter and is read as frontmatter.
pub(crate) fn locate_frontmatter(raw: &str) -> Option<FrontmatterSpan<'_>> {
    let start = if raw.starts_with(BOM) {
        BOM.len_utf8()
    } else {
        0
    };
    let mut lines = raw[start..].split_inclusive('\n');
    let open = lines.next()?;
    if !open.ends_with('\n') || !is_fence(open, false) {
        return None;
    }
    let eol = if open.ends_with("\r\n") { "\r\n" } else { "\n" };
    let yaml_start = start + open.len();
    let mut pos = yaml_start;
    for line in lines {
        if is_fence(line, true) {
            let yaml = &raw[yaml_start..pos];
            return looks_like_frontmatter(yaml).then_some(FrontmatterSpan {
                yaml,
                yaml_start,
                yaml_end: pos,
                body_start: pos + line.len(),
                eol,
            });
        }
        pos += line.len();
    }
    None
}

/// Rule 2 of [`locate_frontmatter`].
fn looks_like_frontmatter(yaml: &str) -> bool {
    if yaml.trim().is_empty() {
        return true;
    }
    match serde_yml::from_str::<serde_yml::Value>(yaml) {
        Ok(value) => value.is_mapping(),
        Err(_) => yaml
            .lines()
            .map(str::trim_end)
            .find(|line| !line.is_empty() && !line.trim_start().starts_with('#'))
            .is_some_and(|line| key_end(line).is_some()),
    }
}

/// If `line` is a top-level YAML mapping entry (`key: value`, `key:`, or a
/// quoted key), the byte offset of its `:`.
pub(crate) fn key_end(line: &str) -> Option<usize> {
    let line = line.trim_end_matches(['\n', '\r']);
    let first = line.chars().next()?;
    let colon = match first {
        '"' | '\'' => {
            let close = line[1..].find(first)? + 1;
            let after = &line[close + 1..];
            let gap = after.len() - after.trim_start_matches([' ', '\t']).len();
            let colon = close + 1 + gap;
            (line[colon..].starts_with(':')).then_some(colon)?
        }
        c if c.is_whitespace() || "#-:?[]{},&*!|>%@`".contains(c) => return None,
        _ => line
            .match_indices(':')
            .map(|(i, _)| i)
            .find(|&i| line[i + 1..].is_empty() || line[i + 1..].starts_with([' ', '\t']))?,
    };
    let after = &line[colon + 1..];
    (after.is_empty() || after.starts_with([' ', '\t'])).then_some(colon)
}

/// Split raw markdown into its YAML frontmatter text and body.
///
/// See [`locate_frontmatter`] for what counts as frontmatter. Returns
/// `(None, raw)` when no frontmatter block is present; the YAML text is
/// returned unparsed so callers can deserialize it into whatever shape they
/// need. The body is everything after the closing fence's line ending,
/// byte for byte.
pub fn split_frontmatter(raw: &str) -> (Option<String>, &str) {
    match locate_frontmatter(raw) {
        Some(span) => (Some(span.yaml.to_string()), &raw[span.body_start..]),
        None => (None, raw),
    }
}

/// Strip YAML frontmatter from `raw`, returning `(Option<Frontmatter>, body)`.
///
/// If frontmatter is absent or malformed, `None` is returned and the
/// remaining text is treated as the body.
pub(super) fn extract_frontmatter(raw: &str) -> (Option<Frontmatter>, &str) {
    let (yaml_text, body) = split_frontmatter(raw);
    let Some(yaml_text) = yaml_text else {
        return (None, body);
    };
    // serde_yml 0.0.13 rejects fully empty documents, so an empty frontmatter
    // block (`---\n---`) is parsed as an empty mapping to keep yielding
    // default frontmatter instead of none.
    let frontmatter = if yaml_text.trim().is_empty() {
        serde_yml::from_str::<Frontmatter>("{}").ok()
    } else {
        serde_yml::from_str::<Frontmatter>(&yaml_text).ok()
    };
    (frontmatter, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;

    #[test]
    fn test_frontmatter_present() {
        let raw = "---\ntags:\n  - rust\n  - markdown\ncreated_at: \"2026-03-27T10:00:00Z\"\nmodified_at: \"2026-03-27T12:00:00Z\"\n---\n# Body";
        let md = Markdown::new(raw);
        let fm = md.frontmatter().expect("frontmatter should be parsed");
        assert_eq!(fm.tags, ["rust", "markdown"]);
        assert!(fm.created_at.is_some());
        assert!(fm.modified_at.is_some());
        assert_eq!(md.body(), "# Body");
    }

    #[test]
    fn test_frontmatter_absent() {
        let raw = "# Just a heading\n\nSome content.";
        let md = Markdown::new(raw);
        assert!(md.frontmatter().is_none());
        assert_eq!(md.body(), raw);
    }

    #[test]
    fn test_frontmatter_partial_fields() {
        let raw = "---\ntags:\n  - notes\n---\nBody text";
        let md = Markdown::new(raw);
        let fm = md.frontmatter().expect("frontmatter should be parsed");
        assert_eq!(fm.tags, ["notes"]);
        assert!(fm.created_at.is_none());
        assert!(fm.modified_at.is_none());
        assert_eq!(md.body(), "Body text");
    }

    #[test]
    fn test_frontmatter_malformed_yaml_with_keys_is_frontmatter() {
        let raw = "---\n# comment\ntags: [unclosed\nicon: x\n---\nBody";
        let md = Markdown::new(raw);
        assert!(md.frontmatter().is_none());
        assert_eq!(md.body(), "Body");
    }

    #[test]
    fn test_fenced_block_that_is_not_a_mapping_is_body() {
        for raw in [
            // Not YAML-looking at all.
            "---\n: invalid: yaml: :\n---\nBody",
            // A paragraph between two rules (a YAML scalar).
            "---\nJust a paragraph between two rules.\n---\n\n# Title\n",
            // A heading between rules (comments only, to YAML).
            "---\n# Heading\n---\nBody",
            // A list between rules.
            "---\n- one\n- two\n---\nBody",
        ] {
            let md = Markdown::new(raw);
            assert!(md.frontmatter().is_none(), "{raw}");
            assert_eq!(md.body(), raw);
        }
    }

    #[test]
    fn test_frontmatter_fences_and_line_endings() {
        let crlf = "---\r\ntags: [a]\r\n---\r\nBody\r\n";
        assert_eq!(Markdown::new(crlf).tags(), ["a"]);
        assert_eq!(Markdown::new(crlf).body(), "Body\r\n");

        let dots = "---\ntags: [a]\n...\nBody";
        assert_eq!(Markdown::new(dots).body(), "Body");

        let bom = "\u{feff}---\ntags: [a]\n---\nBody";
        assert_eq!(Markdown::new(bom).tags(), ["a"]);
        assert_eq!(Markdown::new(bom).body(), "Body");

        // An unclosed block is body; a closing fence at EOF ends the block.
        let unclosed = "---\ntags: [a]\nBody";
        assert_eq!(Markdown::new(unclosed).body(), unclosed);
        assert_eq!(Markdown::new("---\ntags: [a]\n---").body(), "");

        let empty = "---\n---\nBody";
        assert!(Markdown::new(empty).frontmatter().is_some());
        assert_eq!(Markdown::new(empty).body(), "Body");
    }

    #[test]
    fn test_body_block_after_frontmatter_is_body() {
        let raw = "---\ntags: [a]\n---\n---\nkey: value\n---\n\nText";
        assert_eq!(Markdown::new(raw).body(), "---\nkey: value\n---\n\nText");
    }

    #[test]
    fn test_frontmatter_only() {
        let raw = "---\ntags:\n  - empty\n---\n";
        let md = Markdown::new(raw);
        assert!(md.frontmatter().is_some());
        assert_eq!(md.body(), "");
    }

    #[test]
    fn test_frontmatter_not_at_start() {
        let raw = "Some text\n---\ntags:\n  - late\n---\nMore text";
        let md = Markdown::new(raw);
        assert!(md.frontmatter().is_none());
        assert_eq!(md.body(), raw);
    }

    #[test]
    fn test_frontmatter_with_icon() {
        let raw = "---\ntags:\n  - rust\nicon: LuPencil\n---\n# Body";
        let md = Markdown::new(raw);
        let fm = md.frontmatter().expect("frontmatter should be parsed");
        assert_eq!(fm.icon.as_deref(), Some("LuPencil"));
        assert_eq!(fm.tags, ["rust"]);
        assert_eq!(md.body(), "# Body");
    }

    #[test]
    fn test_frontmatter_without_icon() {
        let raw = "---\ntags:\n  - rust\n---\n# Body";
        let md = Markdown::new(raw);
        assert!(md.icon().is_none());
    }

    #[test]
    fn test_icon_shorthand() {
        let raw = "---\nicon: LuPencil\n---\n# Body";
        let md = Markdown::new(raw);
        assert_eq!(md.icon().as_deref(), Some("LuPencil"));
    }

    #[test]
    fn test_favorite_shorthand() {
        let raw = "---\nfavorite: true\n---\n# Body";
        let md = Markdown::new(raw);
        assert_eq!(md.favorite(), Some(true));
    }

    #[test]
    fn test_tags_shorthand() {
        let raw = "---\ntags:\n  - rust\n  - md\n---\n";
        let md = Markdown::new(raw);
        assert_eq!(md.tags(), ["rust", "md"]);
    }

    #[test]
    fn test_tags_empty_when_no_frontmatter() {
        let md = Markdown::new("# Hello");
        assert!(md.tags().is_empty());
    }

    #[test]
    fn test_frontmatter_icon_roundtrip() {
        let fm = Frontmatter {
            tags: vec!["test".to_string()],
            created_at: None,
            modified_at: None,
            icon: Some("LuFolder".to_string()),
            favorite: Some(true),
            presentation: Some("default-dark".to_string()),
        };
        let yaml = serde_yml::to_string(&fm).unwrap();
        let parsed: Frontmatter = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed.icon.as_deref(), Some("LuFolder"));
        assert_eq!(parsed.favorite, Some(true));
        assert_eq!(parsed.presentation.as_deref(), Some("default-dark"));
        assert_eq!(parsed.tags, ["test"]);
    }

    #[test]
    fn test_frontmatter_icon_none_not_serialized() {
        let fm = Frontmatter {
            tags: Vec::new(),
            created_at: None,
            modified_at: None,
            icon: None,
            favorite: None,
            presentation: None,
        };
        let yaml = serde_yml::to_string(&fm).unwrap();
        assert!(
            !yaml.contains("icon"),
            "icon: None should be omitted: {yaml}"
        );
        assert!(
            !yaml.contains("favorite"),
            "favorite: None should be omitted: {yaml}"
        );
        assert!(
            !yaml.contains("presentation"),
            "presentation: None should be omitted: {yaml}"
        );
    }

    #[test]
    fn test_presentation_shorthand() {
        let raw = "---\npresentation: corporate\n---\n# Body";
        let md = Markdown::new(raw);
        assert_eq!(md.presentation().as_deref(), Some("corporate"));
        assert!(Markdown::new("# Body").presentation().is_none());
    }

    #[test]
    fn test_frontmatter_with_favorite() {
        let raw = "---\nfavorite: true\n---\n# Body";
        let (fm, body) = extract_frontmatter(raw);
        let fm = fm.expect("frontmatter should be parsed");
        assert_eq!(fm.favorite, Some(true));
        assert_eq!(body, "# Body");
    }
}
