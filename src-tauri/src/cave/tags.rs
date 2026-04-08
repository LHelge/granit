use super::helpers::note_meta_with_frontmatter;
use super::{Cave, CaveError};
use granit_types::TagMap;
use std::collections::BTreeMap;

impl Cave {
    /// Collect all unique tags across every note in the cave, mapped to the
    /// notes that carry each tag.
    ///
    /// Each note's frontmatter is read to extract its `tags` list. The result
    /// is a `BTreeMap` keyed by tag (alphabetically sorted), where each value
    /// is a `Vec<DocumentMeta>` sorted by slug.
    pub fn list_tags(&self) -> Result<TagMap, CaveError> {
        let mut tags: BTreeMap<String, Vec<granit_types::DocumentMeta>> = BTreeMap::new();

        for abs_path in self.notes.values() {
            let rel = self.relative_path(abs_path);
            let meta = note_meta_with_frontmatter(&rel, abs_path);

            if let Ok(raw) = std::fs::read_to_string(abs_path) {
                let md = crate::markdown::Markdown::new(&raw);
                for tag in md.tags() {
                    tags.entry(tag).or_default().push(meta.clone());
                }
            }
        }

        // Sort notes within each tag by slug (case-insensitive)
        for notes in tags.values_mut() {
            notes.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
        }

        Ok(TagMap { tags })
    }
}
