use chrono::Utc;
use granit_types::Frontmatter;

use super::Markdown;

impl Markdown<'_> {
    /// Generate the initial file content for a new note.
    ///
    /// Produces a YAML frontmatter block with `created_at` and `modified_at`
    /// set to the current UTC time. The note body starts empty.
    pub fn new_note() -> String {
        Self::new_note_with_body("", Vec::new(), None)
    }

    /// Generate initial file content with fresh frontmatter and a provided body.
    pub fn new_note_with_body(body: &str, tags: Vec<String>, icon: Option<String>) -> String {
        let now = Utc::now();
        let fm = Frontmatter {
            tags,
            created_at: Some(now),
            modified_at: Some(now),
            icon,
            favorite: None,
        };
        let yaml = serde_yml::to_string(&fm).unwrap_or_default();
        format!("---\n{yaml}---\n{body}")
    }

    /// Read the existing frontmatter from `existing_raw`, update `modified_at`,
    /// optionally override tags and icon, and prepend it to `new_body`.
    ///
    /// - `tags`: `None` = preserve existing tags; `Some(v)` = replace.
    /// - `icon`: `None` = preserve existing icon; `Some("")` = clear;
    ///   `Some(s)` = set to `s`.
    /// - `favorite`: `None` = preserve existing value; `Some(v)` = set to `v`.
    ///
    /// If the existing content has no parseable frontmatter, a new frontmatter
    /// block is created so legacy notes can gain metadata fields.
    pub fn rebuild(
        existing_raw: &str,
        new_body: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
        favorite: Option<bool>,
    ) -> String {
        let existing = Markdown::new(existing_raw);
        let new = Markdown::new(new_body);
        let body = new.body();
        let should_create_frontmatter = tags.as_ref().is_some_and(|tags| !tags.is_empty())
            || icon.as_deref().is_some_and(|icon| !icon.is_empty())
            || favorite.is_some();
        let now = Utc::now();
        let mut fm = match existing.frontmatter().cloned() {
            Some(fm) => fm,
            None if should_create_frontmatter => Frontmatter {
                tags: Vec::new(),
                created_at: Some(now),
                modified_at: Some(now),
                icon: None,
                favorite: None,
            },
            None => return body.to_string(),
        };
        fm.modified_at = Some(now);
        if let Some(tags) = tags {
            fm.tags = tags;
        }
        if let Some(icon) = icon {
            fm.icon = if icon.is_empty() { None } else { Some(icon) };
        }
        if let Some(favorite) = favorite {
            fm.favorite = Some(favorite);
        }
        let yaml = serde_yml::to_string(&fm).unwrap_or_default();
        format!("---\n{yaml}---\n{body}")
    }
}

#[cfg(test)]
mod tests {
    use crate::markdown::Markdown;

    #[test]
    fn test_rebuild_strips_frontmatter_from_new_body() {
        let existing =
            "---\ntags:\n- original\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nOld body";
        let new_body_with_fm = "---\nsome: injected\n---\nNew body content";
        let result = Markdown::rebuild(existing, new_body_with_fm, None, None, None);
        assert!(result.starts_with("---\n"), "must start with frontmatter");
        assert!(
            result.contains("original"),
            "original tags must be preserved"
        );
        assert!(
            !result.contains("some: injected"),
            "new_body frontmatter must be stripped, got:\n{result}"
        );
        assert!(result.contains("New body content"), "body must be present");
        assert_eq!(
            result.matches("\n---").count(),
            1,
            "only one closing --- expected; got:\n{result}"
        );
    }

    #[test]
    fn test_rebuild_with_tags_override() {
        let existing =
            "---\ntags:\n- old\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", Some(vec!["new".into()]), None, None);
        assert!(result.contains("new"), "new tag must be present");
        assert!(!result.contains("old"), "old tag must be removed");
    }

    #[test]
    fn test_rebuild_with_icon_set_and_clear() {
        let existing =
            "---\ntags: []\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let with_icon = Markdown::rebuild(existing, "Body", None, Some("Star".into()), None);
        assert!(with_icon.contains("Star"), "icon must be set");

        let cleared = Markdown::rebuild(&with_icon, "Body", None, Some(String::new()), None);
        assert!(
            !cleared.contains("Star"),
            "icon must be cleared after empty string"
        );
    }

    #[test]
    fn test_rebuild_no_frontmatter_returns_body_unchanged() {
        let result = Markdown::rebuild("No frontmatter here", "new body", None, None, None);
        assert_eq!(result, "new body");
    }

    #[test]
    fn test_rebuild_preserves_icon() {
        let existing = "---\ntags:\n  - old\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nOld body";
        let result = Markdown::rebuild(existing, "New body", None, None, None);
        assert!(
            result.contains("icon: LuStar"),
            "icon should be preserved: {result}"
        );
        assert!(result.contains("New body"));
    }

    #[test]
    fn test_rebuild_overrides_icon() {
        let existing = "---\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", None, Some("LuFolder".to_string()), None);
        assert!(
            result.contains("icon: LuFolder"),
            "icon should be updated: {result}"
        );
        assert!(
            !result.contains("LuStar"),
            "old icon should be gone: {result}"
        );
    }

    #[test]
    fn test_rebuild_clears_icon() {
        let existing = "---\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", None, Some(String::new()), None);
        assert!(
            !result.contains("icon:"),
            "icon should be cleared: {result}"
        );
    }

    #[test]
    fn test_rebuild_preserves_favorite() {
        let existing = "---\nfavorite: true\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "New body", None, None, None);
        assert!(
            result.contains("favorite: true"),
            "favorite should be preserved: {result}"
        );
        assert!(result.contains("New body"));
    }

    #[test]
    fn test_rebuild_overrides_favorite() {
        let existing = "---\nfavorite: true\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", None, None, Some(false));
        assert!(
            result.contains("favorite: false"),
            "favorite should be updated: {result}"
        );
        assert!(
            !result.contains("favorite: true"),
            "old favorite should be gone: {result}"
        );
    }

    #[test]
    fn test_rebuild_creates_frontmatter_when_missing() {
        let result = Markdown::rebuild(
            "Legacy body",
            "Updated body",
            Some(vec!["legacy".into(), "migrated".into()]),
            Some("Star".into()),
            Some(true),
        );

        assert!(
            result.starts_with("---\n"),
            "frontmatter should be added: {result}"
        );
        assert!(
            result.contains("tags:\n- legacy\n- migrated"),
            "tags should be written: {result}"
        );
        assert!(
            result.contains("icon: Star"),
            "icon should be written: {result}"
        );
        assert!(
            result.contains("favorite: true"),
            "favorite should be written: {result}"
        );
        assert!(
            result.contains("created_at:"),
            "created_at should be initialized: {result}"
        );
        assert!(
            result.contains("modified_at:"),
            "modified_at should be initialized: {result}"
        );
        assert!(
            result.ends_with("Updated body"),
            "body should be preserved: {result}"
        );
    }
}
