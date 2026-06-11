use super::helpers::{
    ensure_md_extension, note_meta_from_relative_path, note_meta_with_frontmatter, validate_name,
    write_atomic, write_new,
};
use super::{Cave, CaveError};
use granit_types::{Document, DocumentMeta};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The outcome of a note-rewrite closure: the new body plus optional
/// frontmatter changes, as accepted by [`crate::markdown::Markdown::rebuild`].
#[derive(Default)]
struct NoteRewrite {
    body: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    favorite: Option<bool>,
}

impl Cave {
    /// Shared read→modify→rebuild→write cycle for note mutations.
    ///
    /// Validates and resolves `slug`, reads the raw file, lets `f` derive the
    /// new body (and optional frontmatter changes) from it, rebuilds the file
    /// preserving remaining frontmatter, writes it atomically, and returns
    /// metadata for the rewritten note. Callers whose change can affect
    /// wiki-links must call `rebuild_link_indexes()` afterwards.
    fn rewrite_note(
        &self,
        slug: &str,
        f: impl FnOnce(&str) -> Result<NoteRewrite, CaveError>,
    ) -> Result<DocumentMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let raw = std::fs::read_to_string(&abs_path)?;
        let rewrite = f(&raw)?;
        let updated = crate::markdown::Markdown::rebuild(
            &raw,
            &rewrite.body,
            rewrite.tags,
            rewrite.icon,
            rewrite.favorite,
        );
        write_atomic(&abs_path, updated.as_str())?;

        let rel = self.relative_path(&abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        let updated_md = crate::markdown::Markdown::new(&updated);
        meta.icon = updated_md.icon();
        meta.favorite = updated_md.favorite();
        Ok(meta)
    }

    /// Create a new note in this cave, optionally inside `folder` (relative path
    /// from cave root) and optionally seeded from a template slug. Returns the
    /// metadata of the created note.
    ///
    /// If `name` is `"untitled"` and the slug already exists anywhere in the cave,
    /// a numeric suffix is appended (`untitled-2`, `untitled-3`, …).
    /// Any other duplicate slug returns `CaveError::AlreadyExists`.
    pub fn create_note(
        &mut self,
        name: &str,
        folder: Option<&Path>,
        template_slug: Option<&str>,
    ) -> Result<DocumentMeta, CaveError> {
        validate_name(name)?;

        let (filename, slug) = self.resolve_new_slug(name)?;

        let daily_config = Self::parse_daily_note_slug(&slug)
            .map(|_| self.load_config())
            .transpose()?;

        let target_dir = self.resolve_target_dir(folder, daily_config.as_ref())?;

        let effective_template_slug = template_slug.map(str::to_string).or_else(|| {
            daily_config
                .as_ref()
                .and_then(|config| config.daily_note_template_slug.clone())
        });

        let final_path = target_dir.join(&filename);
        let body = self.initial_body_for_new_note(&slug, effective_template_slug.as_deref())?;
        let tags = self.initial_tags_for_new_note(effective_template_slug.as_deref())?;
        let icon = self.initial_icon_for_new_note(effective_template_slug.as_deref())?;
        let initial_content = crate::markdown::Markdown::new_note_with_body(&body, tags, icon);
        write_new(&final_path, &initial_content)?;
        self.notes.insert(slug, final_path.clone());
        self.rebuild_link_indexes();

        let rel = self.relative_path(&final_path);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// List all `.md` notes in this cave (recursively), sorted by slug.
    ///
    /// Each note's frontmatter is read to populate fields like `icon` and `favorite`.
    pub fn list_notes(&self) -> Result<Vec<DocumentMeta>, CaveError> {
        let mut notes: Vec<DocumentMeta> = self
            .notes
            .values()
            .map(|abs| note_meta_with_frontmatter(&self.relative_path(abs), abs))
            .collect();
        notes.sort_by_key(|n| n.slug.to_lowercase());
        Ok(notes)
    }

    /// Read a note by slug.
    pub fn read_note(&self, slug: &str) -> Result<Document, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let md = crate::markdown::Markdown::new(&raw);
        let body = md.body().to_string();
        let rel = self.relative_path(abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = md.icon();
        meta.favorite = md.favorite();
        Ok(Document {
            meta,
            content: body,
        })
    }

    /// Read the raw file content of a note (including frontmatter).
    pub fn read_note_raw(&self, slug: &str) -> Result<String, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;
        Ok(std::fs::read_to_string(abs_path)?)
    }

    /// Save new content to an existing note (looked up by slug).
    pub fn save_note(&mut self, slug: &str, content: &str) -> Result<DocumentMeta, CaveError> {
        let meta = self.rewrite_note(slug, |_raw| {
            Ok(NoteRewrite {
                body: content.to_string(),
                ..Default::default()
            })
        })?;
        self.rebuild_link_indexes();
        Ok(meta)
    }

    /// Update only a note's icon while preserving the current body and tags.
    pub fn set_note_icon(
        &self,
        slug: &str,
        icon: Option<String>,
    ) -> Result<DocumentMeta, CaveError> {
        self.rewrite_note(slug, |raw| {
            let md = crate::markdown::Markdown::new(raw);
            Ok(NoteRewrite {
                body: md.body().to_string(),
                icon,
                ..Default::default()
            })
        })
    }

    /// Replace `old_text` with `new_text` in an existing note (looked up by slug).
    /// Fails if `old_text` is not found in the note's content.
    pub fn edit_note(
        &mut self,
        slug: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<DocumentMeta, CaveError> {
        let meta = self.rewrite_note(slug, |raw| {
            let md = crate::markdown::Markdown::new(raw);
            let body = md.body();
            if !body.contains(old_text) {
                return Err(CaveError::EditNotFound);
            }
            Ok(NoteRewrite {
                body: body.replacen(old_text, new_text, 1),
                ..Default::default()
            })
        })?;
        self.rebuild_link_indexes();
        Ok(meta)
    }

    /// Delete a note by slug.
    pub fn delete_note(&mut self, slug: &str) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        std::fs::remove_file(&abs_path)?;
        self.notes.remove(slug);
        self.rebuild_link_indexes();
        Ok(())
    }

    /// Move a note to a different folder within the cave.
    ///
    /// `destination` is a relative path from the cave root (e.g. `Path::new("projects")`).
    /// Passing `None` moves the note to the cave root.
    pub fn move_note(
        &mut self,
        slug: &str,
        destination: Option<&Path>,
    ) -> Result<DocumentMeta, CaveError> {
        validate_name(slug)?;
        let old_abs = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let dest_dir = if let Some(d) = destination {
            super::helpers::validate_folder_path(d)?;
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
            self.check_containment(&abs)?;
            abs
        } else {
            self.path.clone()
        };

        let filename = old_abs
            .file_name()
            .ok_or_else(|| CaveError::InvalidName("note has no filename".into()))?;
        let new_abs = dest_dir.join(filename);

        // No-op if already in the target dir.
        if old_abs == new_abs {
            let rel = self.relative_path(&old_abs);
            return Ok(note_meta_with_frontmatter(&rel, &old_abs));
        }

        // Prevent overwriting an existing file at the destination.
        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&old_abs, &new_abs)?;
        self.notes.insert(slug.to_string(), new_abs.clone());

        let rel = self.relative_path(&new_abs);
        Ok(note_meta_with_frontmatter(&rel, &new_abs))
    }

    /// Rename an existing note in-place (same directory, new filename).
    /// The new slug must not already exist anywhere in the cave.
    pub fn rename_note(
        &mut self,
        old_slug: &str,
        new_name: &str,
    ) -> Result<DocumentMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        // No-op if nothing changed.
        if old_slug == new_name {
            return self.read_note(old_slug).map(|n| n.meta);
        }

        let old_abs = self
            .notes
            .get(old_slug)
            .ok_or_else(|| CaveError::NotFound(old_slug.to_string()))?
            .clone();

        let new_filename = ensure_md_extension(new_name);
        let new_abs = old_abs
            .parent()
            .unwrap_or(Path::new(""))
            .join(&new_filename);

        // Global uniqueness check.
        if self.notes.contains_key(new_name) {
            return Err(CaveError::AlreadyExists(new_filename));
        }

        std::fs::rename(&old_abs, &new_abs)?;
        self.notes.remove(old_slug);
        self.notes.insert(new_name.to_string(), new_abs.clone());
        self.rewrite_wiki_links_for_rename(old_slug, new_name);
        self.rebuild_link_indexes();

        let rel = self.relative_path(&new_abs);
        Ok(note_meta_with_frontmatter(&rel, &new_abs))
    }

    /// Update a note's filename, content, and optionally tags and icon in one operation.
    ///
    /// If `old_slug` and `new_name` differ the file is renamed first (same directory),
    /// then the new content is written. If `tags` is `Some`, the frontmatter tags
    /// are replaced. If `icon` is `Some`, the icon is set (`Some("")` clears it).
    ///
    /// Deliberately not built on [`rewrite_note`]: the rename-then-write here
    /// interleaves filesystem state with a rollback (rename back on write
    /// failure), which a closure-based helper would obscure.
    pub fn update_note(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
        favorite: Option<bool>,
    ) -> Result<DocumentMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        let old_abs = self
            .notes
            .get(old_slug)
            .ok_or_else(|| CaveError::NotFound(old_slug.to_string()))?
            .clone();

        let (final_abs, renamed) = if old_slug != new_name {
            let new_filename = ensure_md_extension(new_name);
            let new_abs = old_abs
                .parent()
                .unwrap_or(Path::new(""))
                .join(&new_filename);

            if self.notes.contains_key(new_name) {
                return Err(CaveError::AlreadyExists(new_filename));
            }

            std::fs::rename(&old_abs, &new_abs)?;
            (new_abs, true)
        } else {
            (old_abs.clone(), false)
        };

        let existing_raw = std::fs::read_to_string(&final_abs)?;
        let updated =
            crate::markdown::Markdown::rebuild(&existing_raw, content, tags, icon, favorite);
        if let Err(e) = write_atomic(&final_abs, updated.as_str()) {
            // Rollback the rename so index stays consistent with filesystem.
            if renamed {
                if let Err(rollback_err) = std::fs::rename(&final_abs, &old_abs) {
                    return Err(CaveError::Io(format!(
                        "failed to write updated note after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e.into());
        }

        // Update index only after all filesystem operations succeed.
        if renamed {
            self.notes.remove(old_slug);
            self.notes.insert(new_name.to_string(), final_abs.clone());
            self.rewrite_wiki_links_for_rename(old_slug, new_name);
        }
        self.rebuild_link_indexes();

        let rel = self.relative_path(&final_abs);
        let mut meta = note_meta_from_relative_path(&rel);
        let updated_md = crate::markdown::Markdown::new(&updated);
        meta.icon = updated_md.icon();
        meta.favorite = updated_md.favorite();
        Ok(meta)
    }

    /// Rewrite wiki-links pointing to `old_slug` across the whole cave so they
    /// point to `new_slug`. Must be called after `self.notes` reflects the
    /// rename, so the renamed note's own self-links are updated too.
    ///
    /// Splices raw bytes (via [`Markdown::rename_wiki_links`]) rather than going
    /// through `Markdown::rebuild`, so each touched note keeps its existing
    /// frontmatter and `modified_at` untouched. Notes with no matching link are
    /// left byte-for-byte unchanged.
    fn rewrite_wiki_links_for_rename(&self, old_slug: &str, new_slug: &str) {
        for abs_path in self.notes.values() {
            let raw = match std::fs::read_to_string(abs_path) {
                Ok(raw) => raw,
                Err(e) => {
                    log::warn!("skipping wiki-link rewrite for {}: {e}", abs_path.display());
                    continue;
                }
            };
            if let Some(updated) =
                crate::markdown::Markdown::rename_wiki_links(&raw, old_slug, new_slug)
            {
                if let Err(e) = write_atomic(abs_path, &updated) {
                    log::warn!(
                        "failed to rewrite wiki-links in {}: {e}",
                        abs_path.display()
                    );
                }
            }
        }
    }

    // ── Backlinks ──────────────────────────────────────────────────

    pub(crate) fn build_backlinks(
        notes: &HashMap<String, std::path::PathBuf>,
        anchors: &HashMap<String, String>,
    ) -> HashMap<String, Vec<String>> {
        let mut backlinks: HashMap<String, HashSet<String>> = HashMap::new();

        for (source_slug, abs_path) in notes {
            let Ok(raw) = std::fs::read_to_string(abs_path) else {
                continue;
            };

            for target_slug in crate::markdown::Markdown::new(&raw)
                .outgoing_links(|name| Self::resolve_link_in(notes, anchors, name))
            {
                if target_slug == *source_slug {
                    continue;
                }

                backlinks
                    .entry(target_slug)
                    .or_default()
                    .insert(source_slug.clone());
            }
        }

        backlinks
            .into_iter()
            .map(|(target_slug, source_slugs)| {
                let mut source_slugs: Vec<String> = source_slugs.into_iter().collect();
                source_slugs.sort_by_key(|slug| slug.to_lowercase());
                (target_slug, source_slugs)
            })
            .collect()
    }

    /// Build the heading-anchor index (anchor id → owning note slug) by scanning
    /// every note body for `{#id}` attributes.
    ///
    /// Anchor ids share the global slug namespace with note slugs, so a
    /// collision (case-insensitive) with a note slug or another anchor is
    /// returned as the second tuple element. The map keeps the first occurrence
    /// in a deterministic (sorted-by-note) order, so callers that ignore the
    /// error still get a consistent index.
    pub(crate) fn collect_anchors(
        notes: &HashMap<String, std::path::PathBuf>,
    ) -> (HashMap<String, String>, Option<CaveError>) {
        let mut anchors: HashMap<String, String> = HashMap::new();
        // Lowercased anchor id → first note that declared it.
        let mut seen_lower: HashMap<String, String> = HashMap::new();
        // Lowercased note slug → canonical note slug, for anchor-vs-note clashes.
        let note_lower: HashMap<String, String> = notes
            .keys()
            .map(|slug| (slug.to_lowercase(), slug.clone()))
            .collect();
        let mut collision: Option<CaveError> = None;

        // Deterministic order so "first wins" is stable across runs.
        let mut sources: Vec<(&String, &std::path::PathBuf)> = notes.iter().collect();
        sources.sort_by_key(|a| a.0.to_lowercase());

        for (note_slug, abs_path) in sources {
            let Ok(raw) = std::fs::read_to_string(abs_path) else {
                continue;
            };
            for anchor_id in crate::markdown::Markdown::new(&raw).anchor_ids() {
                let lower = anchor_id.to_lowercase();
                if let Some(note) = note_lower.get(&lower) {
                    collision.get_or_insert_with(|| CaveError::DuplicateAnchor {
                        slug: anchor_id.clone(),
                        anchor_note: note_slug.clone(),
                        conflict: format!("note \"{note}\""),
                    });
                    continue;
                }
                if let Some(first_note) = seen_lower.get(&lower) {
                    collision.get_or_insert_with(|| CaveError::DuplicateAnchor {
                        slug: anchor_id.clone(),
                        anchor_note: note_slug.clone(),
                        conflict: format!("anchor in \"{first_note}\""),
                    });
                    continue;
                }
                seen_lower.insert(lower, note_slug.clone());
                anchors.insert(anchor_id, note_slug.clone());
            }
        }

        (anchors, collision)
    }

    /// Rebuild the anchor and backlink indexes after a note mutation.
    ///
    /// Anchors are rebuilt leniently (collisions are dropped, first wins) so a
    /// duplicate introduced during live editing never blocks the mutation; the
    /// strict check that refuses to open the cave runs only on a full scan.
    pub(crate) fn rebuild_link_indexes(&mut self) {
        let (anchors, _collision) = Self::collect_anchors(&self.notes);
        self.anchors = anchors;
        self.backlinks = Self::build_backlinks(&self.notes, &self.anchors);
    }

    pub fn backlink_slugs(&self, slug: &str) -> Result<Vec<String>, CaveError> {
        let slug = self.resolve_slug(slug)?;
        Ok(self.backlinks.get(&slug).cloned().unwrap_or_default())
    }

    pub fn backlink_note_metas(&self, slug: &str) -> Result<Vec<DocumentMeta>, CaveError> {
        let backlink_slugs = self.backlink_slugs(slug)?;
        let mut backlinks: Vec<DocumentMeta> = backlink_slugs
            .into_iter()
            .filter_map(|source_slug| {
                self.notes.get(&source_slug).map(|abs_path| {
                    note_meta_with_frontmatter(&self.relative_path(abs_path), abs_path)
                })
            })
            .collect();
        backlinks.sort_by_key(|meta| meta.slug.to_lowercase());
        Ok(backlinks)
    }
}

#[cfg(test)]
mod tests {
    use crate::cave::{Cave, CaveError};
    use granit_types::AppConfig;
    use std::path::Path;

    #[test]
    fn test_cave_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_note("hello", None, None).unwrap();
        assert_eq!(meta.slug, "hello");

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);

        let note = cave.read_note("hello").unwrap();
        assert!(note.content.is_empty() || !note.content.contains("---"));

        cave.save_note("hello", "# Updated\nBody").unwrap();
        let note = cave.read_note("hello").unwrap();
        assert!(note.content.contains("# Updated"));
        assert_eq!(note.meta.slug, "hello");

        cave.rename_note("hello", "world").unwrap();
        assert!(cave.read_note("hello").is_err());
        assert!(cave.read_note("world").is_ok());

        cave.delete_note("world").unwrap();
        assert!(cave.read_note("world").is_err());
    }

    #[test]
    fn test_create_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_note("my-note", None, None).unwrap();
        assert_eq!(meta.slug, "my-note");
        assert_eq!(meta.relative_path, "my-note.md");

        let content = std::fs::read_to_string(dir.path().join("my-note.md")).unwrap();
        assert!(content.contains("created_at"));
    }

    #[test]
    fn test_create_note_in_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir(dir.path().join("notes")).unwrap();

        let meta = cave
            .create_note("nested", Some(Path::new("notes")), None)
            .unwrap();
        assert_eq!(meta.slug, "nested");
        assert_eq!(meta.relative_path, "notes/nested.md");
        assert!(dir.path().join("notes/nested.md").exists());
    }

    #[test]
    fn test_create_note_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .create_note("note", Some(Path::new("nonexistent")), None)
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_note("test", None, None).unwrap();
        let err = cave.create_note("test", None, None).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_note_duplicate_slug_across_folders() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        cave.create_note("foo", None, None).unwrap();
        let err = cave
            .create_note("foo", Some(Path::new("sub")), None)
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_untitled_auto_numbering() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let m1 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m1.slug, "untitled");

        let m2 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m2.slug, "untitled-2");

        let m3 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m3.slug, "untitled-3");
    }

    #[test]
    fn test_create_note_uses_explicit_template_body() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/starter.md"),
            "---\ntags: [template]\nicon: Star\n---\n# {{ slug }}\nBody\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();

        let meta = cave
            .create_note("project-kickoff", None, Some("starter"))
            .unwrap();
        let raw = cave.read_note_raw(&meta.slug).unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(note.content, "# project-kickoff\nBody\n");
        assert_eq!(note.meta.icon.as_deref(), Some("Star"));
        assert_eq!(
            crate::markdown::Markdown::new(&raw).tags(),
            vec!["template".to_string()]
        );
        assert!(raw.contains("icon: Star"));
        assert!(raw.contains("created_at"));
    }

    #[test]
    fn test_create_note_daily_slug_uses_configured_daily_template() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "# {{ date }}\nNext: {{ tomorrow }}\nYesterday: {{ yesterday }}\n{{ weekday }} / {{ weekday_short }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();
        cave.save_config(&AppConfig {
            daily_note_template_slug: Some("daily-template".to_string()),
            ..AppConfig::default()
        })
        .unwrap();

        let meta = cave.create_note("2024-02-29", None, None).unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(meta.relative_path, "Daily/2024-02-29.md");
        assert_eq!(
            note.content,
            "# 2024-02-29\nNext: 2024-03-01\nYesterday: 2024-02-28\nThursday / Thu\n"
        );
    }

    #[test]
    fn test_create_note_explicit_template_overrides_daily_template() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "daily {{ date }}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/manual-template.md"),
            "manual {{ slug }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();
        cave.save_config(&AppConfig {
            daily_note_template_slug: Some("daily-template".to_string()),
            ..AppConfig::default()
        })
        .unwrap();

        let meta = cave
            .create_note("2024-02-29", None, Some("manual-template"))
            .unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(meta.relative_path, "Daily/2024-02-29.md");
        assert_eq!(note.content, "manual 2024-02-29\n");
    }

    #[test]
    fn test_list_notes_recursive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(dir.path().join("sub/beta.md"), "# Beta\n").unwrap();
        std::fs::write(dir.path().join("not-a-note.txt"), "ignore").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 2);
        let slugs: Vec<_> = notes.iter().map(|n| n.slug.as_str()).collect();
        assert!(slugs.contains(&"alpha"));
        assert!(slugs.contains(&"beta"));

        let beta = notes.iter().find(|n| n.slug == "beta").unwrap();
        assert_eq!(beta.relative_path, "sub/beta.md");
    }

    #[test]
    fn test_list_notes_skips_hidden_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::write(dir.path().join("visible.md"), "").unwrap();
        std::fs::write(dir.path().join(".hidden/secret.md"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "visible");
    }

    #[test]
    fn test_list_notes_skips_non_md_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "").unwrap();
        std::fs::write(dir.path().join("image.png"), "").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "note");
    }

    #[test]
    fn test_list_notes_reads_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("favorite.md"),
            "---\nfavorite: true\nicon: Star\n---\nBody",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].favorite, Some(true));
        assert_eq!(notes[0].icon.as_deref(), Some("Star"));
    }

    /// A note whose _content_ cannot be read still appears in list_notes.
    #[test]
    #[cfg(unix)]
    fn test_list_notes_includes_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let unreadable = dir.path().join("locked.md");
        std::fs::write(&unreadable, "secret").unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "locked");

        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn test_read_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Test Note\nBody").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("test").unwrap();
        assert_eq!(note.meta.slug, "test");
        assert!(note.content.contains("Body"));
    }

    #[test]
    fn test_read_note_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/deep.md"), "# Deep\nContent").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("deep").unwrap();
        assert_eq!(note.meta.slug, "deep");
        assert_eq!(note.meta.relative_path, "sub/deep.md");
    }

    #[test]
    fn test_read_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.read_note("nonexistent").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_read_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.read_note("../etc/passwd").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_read_note_reads_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("favorite.md"),
            "---\nfavorite: false\n---\nBody",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("favorite").unwrap();
        assert_eq!(note.meta.favorite, Some(false));
        assert_eq!(note.content, "Body");
    }

    #[test]
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.save_note("test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("New body"));
    }

    #[test]
    fn test_save_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("missing", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_save_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("../escape", "content").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_set_note_icon_preserves_existing_body() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("note.md");
        std::fs::write(&note_path, "---\ntags: [test]\n---\nBody text\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .set_note_icon("note", Some("Star".to_string()))
            .unwrap();
        let note = cave.read_note("note").unwrap();

        assert_eq!(meta.icon.as_deref(), Some("Star"));
        assert_eq!(note.meta.icon.as_deref(), Some("Star"));
        assert_eq!(note.content, "Body text\n");
    }

    #[test]
    fn test_edit_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nold text here").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.edit_note("test", "old text", "new text").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("new text here"));
        assert!(!content.contains("old text"));
    }

    #[test]
    fn test_edit_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.edit_note("missing", "old", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_edit_note_text_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nsome content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .edit_note("test", "nonexistent text", "replacement")
            .unwrap_err();
        assert!(matches!(err, CaveError::EditNotFound));
    }

    #[test]
    fn test_edit_note_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "AAA BBB AAA").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.edit_note("test", "AAA", "CCC").unwrap();
        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert_eq!(content, "CCC BBB AAA");
    }

    #[test]
    fn test_delete_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("doomed.md"), "# Bye\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("doomed.md").exists());

        cave.delete_note("doomed").unwrap();
        assert!(!dir.path().join("doomed.md").exists());
    }

    #[test]
    fn test_delete_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_note("ghost").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_note("../escape").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    // ── move_note ──────────────────────────────────────────────────

    #[test]
    fn test_move_note_to_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# Note").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", Some(Path::new("sub"))).unwrap();
        assert_eq!(meta.slug, "note");
        assert_eq!(meta.relative_path, "sub/note.md");
        assert!(!dir.path().join("note.md").exists());
        assert!(dir.path().join("sub/note.md").exists());
    }

    #[test]
    fn test_move_note_to_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", None).unwrap();
        assert_eq!(meta.relative_path, "note.md");
        assert!(dir.path().join("note.md").exists());
        assert!(!dir.path().join("sub/note.md").exists());
    }

    #[test]
    fn test_move_note_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", Some(Path::new("sub"))).unwrap();
        assert_eq!(meta.relative_path, "sub/note.md");
    }

    #[test]
    fn test_move_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.move_note("ghost", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_note_dest_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_note("note", Some(Path::new("nonexistent")))
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_note_file_already_exists_at_dest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.create_note("note", None, None).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "conflict").unwrap();

        let err = cave.move_note("note", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    // ── rename_note ────────────────────────────────────────────────

    #[test]
    fn test_rename_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "# Old Note\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.rename_note("old", "new-name").unwrap();
        assert_eq!(meta.slug, "new-name");
        assert_eq!(meta.relative_path, "new-name.md");
        assert!(!dir.path().join("old.md").exists());
        assert!(dir.path().join("new-name.md").exists());
    }

    #[test]
    fn test_rename_note_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("same.md"), "# Same\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.rename_note("same", "same").unwrap();
        assert_eq!(meta.slug, "same");
        assert!(dir.path().join("same.md").exists());
    }

    #[test]
    fn test_rename_note_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_target_exists_in_other_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("sub/b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("missing", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_rename_note_updates_inbound_wiki_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(
            dir.path().join("source.md"),
            "See [[target]] and [[target|the target]].\n",
        )
        .unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("target", "destination").unwrap();

        let source = std::fs::read_to_string(dir.path().join("source.md")).unwrap();
        assert_eq!(
            source,
            "See [[destination]] and [[destination|the target]].\n"
        );
        assert_eq!(
            cave.backlink_slugs("destination").unwrap(),
            vec!["source".to_string()]
        );
    }

    #[test]
    fn test_rename_note_updates_self_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.md"), "I link to [[foo]] myself.\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("foo", "bar").unwrap();

        let bar = std::fs::read_to_string(dir.path().join("bar.md")).unwrap();
        assert_eq!(bar, "I link to [[bar]] myself.\n");
    }

    #[test]
    fn test_rename_note_leaves_code_block_links_intact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(
            dir.path().join("source.md"),
            "```\n[[target]]\n```\nreal [[target]]\n",
        )
        .unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("target", "renamed").unwrap();

        let source = std::fs::read_to_string(dir.path().join("source.md")).unwrap();
        assert_eq!(source, "```\n[[target]]\n```\nreal [[renamed]]\n");
    }

    #[test]
    fn test_rename_note_leaves_unrelated_note_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        let other = "---\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nLinks to [[elsewhere]].\n";
        std::fs::write(dir.path().join("other.md"), other).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("target", "renamed").unwrap();

        let after = std::fs::read_to_string(dir.path().join("other.md")).unwrap();
        assert_eq!(after, other);
    }

    // ── update_note ────────────────────────────────────────────────

    #[test]
    fn test_update_note_content_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "old content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("note", "note", "new content", None, None, None)
            .unwrap();
        assert_eq!(meta.slug, "note");

        let saved = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert_eq!(saved, "new content");
    }

    #[test]
    fn test_update_note_rename_and_save() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "original content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("old", "new-name", "updated content", None, None, None)
            .unwrap();
        assert_eq!(meta.slug, "new-name");
        assert!(!dir.path().join("old.md").exists());

        let saved = std::fs::read_to_string(dir.path().join("new-name.md")).unwrap();
        assert_eq!(saved, "updated content");
    }

    #[test]
    fn test_update_note_rename_updates_inbound_wiki_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "original content").unwrap();
        std::fs::write(dir.path().join("source.md"), "See [[old]].\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.update_note("old", "new-name", "updated content", None, None, None)
            .unwrap();

        let source = std::fs::read_to_string(dir.path().join("source.md")).unwrap();
        assert_eq!(source, "See [[new-name]].\n");
    }

    #[test]
    fn test_update_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .update_note("ghost", "ghost", "content", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_update_note_rename_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "a content").unwrap();
        std::fs::write(dir.path().join("b.md"), "b content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .update_note("a", "b", "new content", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
        let content = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(content, "a content");
    }

    #[test]
    fn test_update_note_sets_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "---\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody",
        )
        .unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("note", "note", "Updated", None, None, Some(true))
            .unwrap();
        let raw = std::fs::read_to_string(dir.path().join("note.md")).unwrap();

        assert_eq!(meta.favorite, Some(true));
        assert!(
            raw.contains("favorite: true"),
            "favorite should be persisted: {raw}"
        );
        assert!(raw.contains("Updated"), "body should be updated: {raw}");
    }

    #[test]
    fn test_update_note_adds_frontmatter_to_legacy_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("legacy.md"), "Legacy body").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note(
                "legacy",
                "legacy",
                "Updated body",
                Some(vec!["legacy".into(), "migrated".into()]),
                Some("Star".into()),
                Some(true),
            )
            .unwrap();
        let raw = std::fs::read_to_string(dir.path().join("legacy.md")).unwrap();

        assert_eq!(meta.icon.as_deref(), Some("Star"));
        assert_eq!(meta.favorite, Some(true));
        assert!(
            raw.starts_with("---\n"),
            "frontmatter should be added: {raw}"
        );
        assert!(
            raw.contains("- legacy") && raw.contains("- migrated"),
            "tags should be persisted: {raw}"
        );
        assert!(
            raw.contains("icon: Star"),
            "icon should be persisted: {raw}"
        );
        assert!(
            raw.contains("favorite: true"),
            "favorite should be persisted: {raw}"
        );
        assert!(
            raw.contains("created_at:"),
            "created_at should be initialized: {raw}"
        );
        assert!(
            raw.contains("modified_at:"),
            "modified_at should be initialized: {raw}"
        );
        assert!(
            raw.ends_with("Updated body"),
            "body should be updated: {raw}"
        );
        assert_eq!(
            crate::markdown::Markdown::new(&raw).tags(),
            vec!["legacy", "migrated"]
        );
    }

    // ── backlinks ──────────────────────────────────────────────────

    #[test]
    fn test_open_builds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source-a.md"), "[[target]]\n").unwrap();
        std::fs::write(dir.path().join("source-b.md"), "[[target]]\n").unwrap();

        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source-a".to_string(), "source-b".to_string()]
        );
    }

    #[test]
    fn test_backlinks_deduplicate_repeated_links_from_same_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(
            dir.path().join("source.md"),
            "[[target]] and [[target|again]]\n",
        )
        .unwrap();

        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source".to_string()]
        );
    }

    #[test]
    fn test_save_note_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "No links yet\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());

        cave.save_note("source", "[[target]]\n").unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source".to_string()]
        );
    }

    #[test]
    fn test_rename_note_rebuilds_backlink_source_slug() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("source", "renamed-source").unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["renamed-source".to_string()]
        );
    }

    #[test]
    fn test_delete_note_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.delete_note("source").unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());
    }

    #[test]
    fn test_delete_folder_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("sub/source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.delete_folder(Path::new("sub")).unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());
    }
}
