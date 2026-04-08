use granit_types::Frontmatter;
use pulldown_cmark::{Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};

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

    /// Shorthand: extract the `tags` field from frontmatter.
    pub fn tags(&self) -> Vec<String> {
        self.frontmatter()
            .map(|fm| fm.tags.clone())
            .unwrap_or_default()
    }
}

/// Strip YAML frontmatter from `raw`, returning `(Option<Frontmatter>, body)`.
///
/// Uses pulldown-cmark's `ENABLE_YAML_STYLE_METADATA_BLOCKS` to detect
/// frontmatter delimited by `---` (or `...`) at the very start of the
/// document. If absent or malformed, `None` is returned and the remaining
/// text is treated as the body.
pub(super) fn extract_frontmatter(raw: &str) -> (Option<Frontmatter>, &str) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let mut yaml_text = String::new();
    let mut found_metadata = false;
    let mut body_start: usize = 0;

    for (event, range) in Parser::new_ext(raw, options).into_offset_iter() {
        match event {
            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                found_metadata = true;
            }
            Event::Text(text) if found_metadata && body_start == 0 => {
                yaml_text.push_str(&text);
            }
            Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                body_start = range.end;
                break;
            }
            _ if !found_metadata => {
                return (None, raw);
            }
            _ => {}
        }
    }

    if !found_metadata {
        return (None, raw);
    }

    let body = &raw[body_start..];
    // Skip the newline after the closing fence (pulldown-cmark's range ends
    // right after `---` / `...` but does not consume the line ending).
    let body = body
        .strip_prefix('\n')
        .or_else(|| body.strip_prefix("\r\n"))
        .unwrap_or(body);
    let frontmatter = serde_yml::from_str::<Frontmatter>(&yaml_text).ok();
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
    fn test_frontmatter_malformed_yaml() {
        let raw = "---\n: invalid: yaml: :\n---\nBody";
        let md = Markdown::new(raw);
        assert!(md.frontmatter().is_none());
        assert_eq!(md.body(), "Body");
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
        };
        let yaml = serde_yml::to_string(&fm).unwrap();
        let parsed: Frontmatter = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed.icon.as_deref(), Some("LuFolder"));
        assert_eq!(parsed.favorite, Some(true));
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
