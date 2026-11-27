//! What a note header shows and edits: the owned frontmatter fields (tags,
//! icon, favorite), the dates, word count and reading time, and the
//! backlinks — the same data the Tauri reader takes from `render_note`,
//! without rendering the body to HTML.

use super::helpers::validate_name;
use super::{Cave, CaveError};
use crate::markdown::{DocumentStats, Markdown};
use granit_types::DocumentMeta;

/// The header of a note or note template.
#[derive(Debug, Clone, Default)]
pub struct DocumentHeader {
    /// Frontmatter `tags`, in file order.
    pub tags: Vec<String>,
    /// Frontmatter `icon` id (`None` when absent or empty).
    pub icon: Option<String>,
    /// Frontmatter `favorite`; always `false` for templates.
    pub favorite: bool,
    pub stats: DocumentStats,
    /// The notes linking here, sorted by slug; always empty for templates
    /// (their slugs are a namespace of their own).
    pub backlinks: Vec<DocumentMeta>,
}

/// One change made in a header, to one owned frontmatter key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderEdit {
    /// Add a tag, normalized by [`normalize_tag`]; a blank or present tag
    /// changes nothing.
    AddTag(String),
    /// Remove every occurrence of the tag.
    RemoveTag(String),
    /// Set the icon id; `None` removes the key.
    Icon(Option<String>),
    /// Set the favorite flag. Templates have none: it changes nothing there.
    Favorite(bool),
}

/// The frontmatter changes of an edit, as [`Markdown::rebuild`] takes them.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct FieldChanges {
    pub tags: Option<Vec<String>>,
    pub icon: Option<String>,
    pub favorite: Option<bool>,
}

/// A typed tag as it is stored: trimmed and lowercased; `None` when blank.
pub fn normalize_tag(raw: &str) -> Option<String> {
    let tag = raw.trim().to_lowercase();
    (!tag.is_empty()).then_some(tag)
}

impl HeaderEdit {
    /// The changes this edit makes to the document `md`, or `None` when it
    /// changes nothing.
    pub(super) fn changes(&self, md: &Markdown) -> Option<FieldChanges> {
        let mut changes = FieldChanges::default();
        match self {
            HeaderEdit::AddTag(raw) => {
                let tag = normalize_tag(raw)?;
                let mut tags = md.tags();
                if tags.contains(&tag) {
                    return None;
                }
                tags.push(tag);
                changes.tags = Some(tags);
            }
            HeaderEdit::RemoveTag(tag) => {
                let mut tags = md.tags();
                let before = tags.len();
                tags.retain(|existing| existing != tag);
                if tags.len() == before {
                    return None;
                }
                changes.tags = Some(tags);
            }
            // `Some("")` is how `rebuild` removes the key.
            HeaderEdit::Icon(icon) => changes.icon = Some(icon.clone().unwrap_or_default()),
            HeaderEdit::Favorite(favorite) => changes.favorite = Some(*favorite),
        }
        Some(changes)
    }
}

impl DocumentHeader {
    fn of(md: &Markdown, backlinks: Vec<DocumentMeta>, favorite: bool) -> Self {
        DocumentHeader {
            tags: md.tags(),
            icon: md.icon().filter(|icon| !icon.is_empty()),
            favorite: favorite && md.favorite().unwrap_or(false),
            stats: md.stats(),
            backlinks,
        }
    }
}

impl Cave {
    /// The header of the note `slug`, from the file on disk.
    pub fn note_header(&self, slug: &str) -> Result<DocumentHeader, CaveError> {
        let raw = self.read_note_raw(slug)?;
        let backlinks = self.backlink_note_metas(slug)?;
        Ok(DocumentHeader::of(&Markdown::new(&raw), backlinks, true))
    }

    /// The header of the note template `slug`, from the file on disk.
    pub fn template_header(&self, slug: &str) -> Result<DocumentHeader, CaveError> {
        validate_name(slug)?;
        let raw = self.read_template_raw(slug)?;
        Ok(DocumentHeader::of(&Markdown::new(&raw), Vec::new(), false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_trimmed_and_lowercased() {
        assert_eq!(normalize_tag("  Work Log "), Some("work log".into()));
        assert_eq!(normalize_tag(" \t "), None);
    }

    #[test]
    fn edits_change_only_their_own_field() {
        let raw = "---\ntags: [a]\nicon: Star\n---\nbody\n";
        let md = Markdown::new(raw);
        assert_eq!(
            HeaderEdit::AddTag(" B ".into()).changes(&md),
            Some(FieldChanges {
                tags: Some(vec!["a".into(), "b".into()]),
                ..Default::default()
            })
        );
        assert_eq!(HeaderEdit::AddTag("A".into()).changes(&md), None);
        assert_eq!(HeaderEdit::AddTag("  ".into()).changes(&md), None);
        assert_eq!(
            HeaderEdit::RemoveTag("a".into()).changes(&md),
            Some(FieldChanges {
                tags: Some(Vec::new()),
                ..Default::default()
            })
        );
        assert_eq!(HeaderEdit::RemoveTag("zz".into()).changes(&md), None);
        assert_eq!(
            HeaderEdit::Icon(None).changes(&md),
            Some(FieldChanges {
                icon: Some(String::new()),
                ..Default::default()
            })
        );
        assert_eq!(
            HeaderEdit::Favorite(true).changes(&md),
            Some(FieldChanges {
                favorite: Some(true),
                ..Default::default()
            })
        );
    }

    #[test]
    fn a_header_edit_splices_only_its_key_and_keeps_the_body() {
        let dir = tempfile::tempdir().unwrap();
        let raw = "---\n# mine\ncreated_at: 2026-01-01T00:00:00Z\ntags:\n  - a\ncustom: \"x\"\n---\nbody [[b]]\n";
        std::fs::write(dir.path().join("n.md"), raw).unwrap();
        std::fs::write(dir.path().join("b.md"), "see [[n]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_scanned().unwrap();

        let meta = cave
            .edit_note_header("n", HeaderEdit::Favorite(true))
            .unwrap();
        assert_eq!(meta.favorite, Some(true));
        let written = std::fs::read_to_string(dir.path().join("n.md")).unwrap();
        assert!(
            written.starts_with(
                "---\n# mine\ncreated_at: 2026-01-01T00:00:00Z\ntags:\n  - a\ncustom: \"x\"\n"
            ),
            "{written}"
        );
        assert!(written.contains("\nfavorite: true\n"), "{written}");
        assert!(written.contains("\nmodified_at: "), "{written}");
        assert!(written.ends_with("---\nbody [[b]]\n"), "{written}");

        // An edit that changes nothing writes nothing.
        cave.edit_note_header("n", HeaderEdit::AddTag("A".into()))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("n.md")).unwrap(),
            written
        );

        let header = cave.note_header("n").unwrap();
        assert_eq!(header.tags, vec!["a"]);
        assert!(header.favorite);
        assert_eq!(header.stats.word_count, 2);
        assert_eq!(header.stats.reading_minutes, 1);
        assert!(header.stats.created_display.is_some());
        assert!(header.stats.modified_display.is_some());
        let backlinks: Vec<_> = header.backlinks.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(backlinks, vec!["b"]);
    }

    #[test]
    fn unknown_or_invalid_slugs_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_scanned().unwrap();
        let edit = || HeaderEdit::Favorite(true);

        assert!(matches!(
            cave.edit_note_header("missing", edit()),
            Err(CaveError::NotFound(_))
        ));
        assert!(matches!(
            cave.note_header("missing"),
            Err(CaveError::NotFound(_))
        ));
        assert!(matches!(
            cave.edit_template_header("missing", HeaderEdit::Icon(None)),
            Err(CaveError::TemplateNotFound(_))
        ));
        assert!(matches!(
            cave.template_header("missing"),
            Err(CaveError::TemplateNotFound(_))
        ));
        for slug in ["../x", ".hidden", ""] {
            assert!(
                matches!(
                    cave.edit_note_header(slug, edit()),
                    Err(CaveError::InvalidName(_))
                ),
                "{slug}"
            );
            assert!(
                matches!(
                    cave.edit_template_header(slug, edit()),
                    Err(CaveError::InvalidName(_))
                ),
                "{slug}"
            );
            assert!(
                matches!(cave.template_header(slug), Err(CaveError::InvalidName(_))),
                "{slug}"
            );
        }
        assert!(!dir.path().parent().unwrap().join("x.md").exists());
    }

    #[test]
    fn a_template_header_edits_icon_and_tags_but_has_no_favorite() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_scanned().unwrap();
        cave.create_template("t").unwrap();
        cave.save_template("t", "# {{ slug }}\n").unwrap();
        let path = dir.path().join(".granit/templates/t.md");
        let before = std::fs::read_to_string(&path).unwrap();

        cave.edit_template_header("t", HeaderEdit::Favorite(true))
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let meta = cave
            .edit_template_header("t", HeaderEdit::Icon(Some("Star".into())))
            .unwrap();
        assert_eq!(meta.icon.as_deref(), Some("Star"));
        cave.edit_template_header("t", HeaderEdit::AddTag("Daily".into()))
            .unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.ends_with("---\n# {{ slug }}\n"), "{after}");

        let header = cave.template_header("t").unwrap();
        assert_eq!(header.icon.as_deref(), Some("Star"));
        assert_eq!(header.tags, vec!["daily"]);
        assert!(!header.favorite);
        assert!(header.backlinks.is_empty());
        assert!(header.stats.created_display.is_some());
    }
}
