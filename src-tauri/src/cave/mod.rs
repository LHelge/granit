mod error;
mod note;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use error::CaveError;
pub use note::{Note, NoteMeta};

use note::{
    ensure_md_extension, note_meta_from_relative_path, validate_folder_path, validate_name,
};

/// A cave — an open directory of markdown notes.
pub struct Cave {
    path: PathBuf,
    /// In-memory index: slug → absolute path. Populated at open and kept in
    /// sync by create / delete / rename / update operations.
    /// Slug uniqueness is enforced globally across all subdirectories.
    notes: HashMap<String, PathBuf>,
}

impl Cave {
    /// Open a cave at the given directory path. Eagerly scans recursively for
    /// `.md` files to populate the in-memory notes index.
    pub fn open(path: PathBuf) -> Self {
        let notes = Self::scan_recursive(&path, &path).unwrap_or_default();
        Self { path, notes }
    }

    /// Recursively scan `dir` for `.md` files and return a slug → absolute-path map.
    ///
    /// Subdirectories starting with `.` (hidden) are skipped, as is `.granit/`.
    /// If two files share the same filename (slug) the second one is skipped with
    /// a logged warning — first one encountered wins.
    #[allow(clippy::map_entry)]
    fn scan_recursive(
        _cave_root: &Path,
        dir: &Path,
    ) -> Result<HashMap<String, PathBuf>, CaveError> {
        let mut notes = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if p.is_dir() {
                // Skip hidden dirs and the cave config dir.
                if name_str.starts_with('.') || name_str == ".granit" {
                    continue;
                }
                let sub = Self::scan_recursive(_cave_root, &p)?;
                for (slug, abs_path) in sub {
                    if notes.contains_key(&slug) {
                        eprintln!(
                            "granit: duplicate slug {slug:?} found at {abs_path:?}, skipping"
                        );
                    } else {
                        notes.insert(slug, abs_path);
                    }
                }
            } else if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext == "md" {
                        let slug = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        if notes.contains_key(&slug) {
                            eprintln!("granit: duplicate slug {slug:?} found at {p:?}, skipping");
                        } else {
                            notes.insert(slug, p);
                        }
                    }
                }
            }
        }
        Ok(notes)
    }

    /// Return the relative path from `self.path` to `abs_path` as a `PathBuf`.
    fn relative_path(&self, abs_path: &Path) -> PathBuf {
        abs_path
            .strip_prefix(&self.path)
            .unwrap_or(abs_path)
            .to_path_buf()
    }

    /// Look up a note slug by name (case-insensitive).
    ///
    /// Returns the stored slug if found, `None` otherwise. Designed to be passed
    /// as a closure to `markdown::resolve_wiki_links`.
    pub fn lookup_slug(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.notes
            .keys()
            .find(|k| k.to_lowercase() == lower)
            .map(String::as_str)
    }

    /// The root directory of this cave.
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a new note in this cave, optionally inside `folder` (relative path
    /// from cave root). Returns the metadata of the created note.
    ///
    /// If `name` is `"untitled"` and the slug already exists anywhere in the cave,
    /// a numeric suffix is appended (`untitled-2`, `untitled-3`, …).
    /// Any other duplicate slug returns `CaveError::AlreadyExists`.
    pub fn create_note(
        &mut self,
        name: &str,
        folder: Option<&Path>,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(name)?;

        // Resolve the target directory.
        let target_dir = if let Some(f) = folder {
            validate_folder_path(f)?;
            let d = self.path.join(f);
            if !d.is_dir() {
                return Err(CaveError::NotFound(f.to_string_lossy().into_owned()));
            }
            d
        } else {
            self.path.clone()
        };

        let base_filename = ensure_md_extension(name);

        // For "untitled" auto-numbering, uniqueness is checked globally via the index.
        let (filename, slug) = if name == "untitled" && self.notes.contains_key("untitled") {
            let mut n = 2u32;
            loop {
                let candidate_slug = format!("untitled-{n}");
                let candidate_file = format!("{candidate_slug}.md");
                if !self.notes.contains_key(&candidate_slug) {
                    break (candidate_file, candidate_slug);
                }
                n += 1;
            }
        } else if self.notes.contains_key(name) {
            return Err(CaveError::AlreadyExists(base_filename));
        } else {
            (base_filename, name.to_string())
        };

        let final_path = target_dir.join(&filename);
        let initial_content = crate::markdown::initial_content(&slug);
        std::fs::write(&final_path, &initial_content)?;
        self.notes.insert(slug, final_path.clone());

        let rel = self.relative_path(&final_path);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// Create a subdirectory within the cave.
    ///
    /// `path` is a relative path from the cave root (e.g. `"notes"` or `"projects/2026"`).
    #[allow(dead_code)]
    pub fn create_folder(&mut self, path: &str) -> Result<(), CaveError> {
        validate_folder_path(Path::new(path))?;
        let abs = self.path.join(path);
        if abs.exists() {
            return Err(CaveError::AlreadyExists(path.to_string()));
        }
        std::fs::create_dir_all(&abs)?;
        Ok(())
    }

    /// List all `.md` notes in this cave (recursively), sorted by slug.
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, CaveError> {
        let mut notes: Vec<NoteMeta> = self
            .notes
            .values()
            .map(|abs| note_meta_from_relative_path(&self.relative_path(abs)))
            .collect();
        notes.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
        Ok(notes)
    }

    /// Read a note by slug.
    pub fn read_note(&self, slug: &str) -> Result<Note, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let content = std::fs::read_to_string(abs_path)?;
        let rel = self.relative_path(abs_path);
        Ok(Note {
            meta: note_meta_from_relative_path(&rel),
            content,
        })
    }

    /// Save new content to an existing note (looked up by slug).
    pub fn save_note(&self, slug: &str, content: &str) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let updated = crate::markdown::update_modified_at(content);
        std::fs::write(abs_path, updated.as_str())?;
        let rel = self.relative_path(abs_path);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// Replace `old_text` with `new_text` in an existing note (looked up by slug).
    /// Fails if `old_text` is not found in the note's content.
    #[allow(dead_code)]
    pub fn edit_note(
        &self,
        slug: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let content = std::fs::read_to_string(&abs_path)?;
        if !content.contains(old_text) {
            return Err(CaveError::EditNotFound);
        }
        let new_content = content.replacen(old_text, new_text, 1);
        std::fs::write(&abs_path, &new_content)?;

        let rel = self.relative_path(&abs_path);
        Ok(note_meta_from_relative_path(&rel))
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
        Ok(())
    }

    /// Rename an existing note in-place (same directory, new filename).
    /// The new slug must not already exist anywhere in the cave.
    pub fn rename_note(&mut self, old_slug: &str, new_name: &str) -> Result<NoteMeta, CaveError> {
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

        let rel = self.relative_path(&new_abs);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// Update a note's filename and content in one operation.
    ///
    /// If `old_slug` and `new_name` differ the file is renamed first (same directory),
    /// then the new content is written.
    pub fn update_note(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        let old_abs = self
            .notes
            .get(old_slug)
            .ok_or_else(|| CaveError::NotFound(old_slug.to_string()))?
            .clone();

        let final_abs = if old_slug != new_name {
            let new_filename = ensure_md_extension(new_name);
            let new_abs = old_abs
                .parent()
                .unwrap_or(Path::new(""))
                .join(&new_filename);

            if self.notes.contains_key(new_name) {
                return Err(CaveError::AlreadyExists(new_filename));
            }

            std::fs::rename(&old_abs, &new_abs)?;
            self.notes.remove(old_slug);
            self.notes.insert(new_name.to_string(), new_abs.clone());
            new_abs
        } else {
            old_abs
        };

        let updated = crate::markdown::update_modified_at(content);
        std::fs::write(&final_abs, updated.as_str())?;

        let rel = self.relative_path(&final_abs);
        Ok(note_meta_from_relative_path(&rel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cave_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.create_note("hello", None).unwrap();
        assert_eq!(meta.slug, "hello");

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);

        let note = cave.read_note("hello").unwrap();
        assert!(note.content.contains("# hello"));

        cave.save_note("hello", "# Updated\nBody").unwrap();
        let note = cave.read_note("hello").unwrap();
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
        let mut cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.create_note("my-note", None).unwrap();
        assert_eq!(meta.slug, "my-note");
        assert_eq!(meta.relative_path, "my-note.md");

        let content = std::fs::read_to_string(dir.path().join("my-note.md")).unwrap();
        assert!(content.contains("# my-note"));
    }

    #[test]
    fn test_create_note_in_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());
        std::fs::create_dir(dir.path().join("notes")).unwrap();

        let meta = cave
            .create_note("nested", Some(Path::new("notes")))
            .unwrap();
        assert_eq!(meta.slug, "nested");
        assert_eq!(meta.relative_path, "notes/nested.md");
        assert!(dir.path().join("notes/nested.md").exists());
    }

    #[test]
    fn test_create_note_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave
            .create_note("note", Some(Path::new("nonexistent")))
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        cave.create_note("test", None).unwrap();
        let err = cave.create_note("test", None).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_note_duplicate_slug_across_folders() {
        // Two notes with the same filename in different folders should be rejected.
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        cave.create_note("foo", None).unwrap();
        let err = cave.create_note("foo", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_untitled_auto_numbering() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let m1 = cave.create_note("untitled", None).unwrap();
        assert_eq!(m1.slug, "untitled");

        let m2 = cave.create_note("untitled", None).unwrap();
        assert_eq!(m2.slug, "untitled-2");

        let m3 = cave.create_note("untitled", None).unwrap();
        assert_eq!(m3.slug, "untitled-3");
    }

    #[test]
    fn test_create_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        cave.create_folder("projects").unwrap();
        assert!(dir.path().join("projects").is_dir());
    }

    #[test]
    fn test_create_folder_nested() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        cave.create_folder("a/b/c").unwrap();
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn test_create_folder_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        std::fs::create_dir(dir.path().join("existing")).unwrap();
        let err = cave.create_folder("existing").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_list_notes_recursive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(dir.path().join("sub/beta.md"), "# Beta\n").unwrap();
        std::fs::write(dir.path().join("not-a-note.txt"), "ignore").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

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
        let cave = Cave::open(dir.path().to_path_buf());

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
        let cave = Cave::open(dir.path().to_path_buf());

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "note");
    }

    #[test]
    fn test_read_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Test Note\nBody").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let note = cave.read_note("test").unwrap();
        assert_eq!(note.meta.slug, "test");
        assert!(note.content.contains("Body"));
    }

    #[test]
    fn test_read_note_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/deep.md"), "# Deep\nContent").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let note = cave.read_note("deep").unwrap();
        assert_eq!(note.meta.slug, "deep");
        assert_eq!(note.meta.relative_path, "sub/deep.md");
    }

    #[test]
    fn test_read_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.read_note("nonexistent").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_read_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.read_note("../etc/passwd").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.save_note("test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("New body"));
    }

    #[test]
    fn test_save_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.save_note("missing", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_save_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.save_note("../escape", "content").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_edit_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nold text here").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.edit_note("test", "old text", "new text").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("new text here"));
        assert!(!content.contains("old text"));
    }

    #[test]
    fn test_edit_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.edit_note("missing", "old", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_edit_note_text_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nsome content").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave
            .edit_note("test", "nonexistent text", "replacement")
            .unwrap_err();
        assert!(matches!(err, CaveError::EditNotFound));
    }

    #[test]
    fn test_edit_note_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "AAA BBB AAA").unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        cave.edit_note("test", "AAA", "CCC").unwrap();
        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert_eq!(content, "CCC BBB AAA");
    }

    #[test]
    fn test_delete_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("doomed.md"), "# Bye\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());
        assert!(dir.path().join("doomed.md").exists());

        cave.delete_note("doomed").unwrap();
        assert!(!dir.path().join("doomed.md").exists());
    }

    #[test]
    fn test_delete_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.delete_note("ghost").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.delete_note("../escape").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_rename_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "# Old Note\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

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
        let mut cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.rename_note("same", "same").unwrap();
        assert_eq!(meta.slug, "same");
        assert!(dir.path().join("same.md").exists());
    }

    #[test]
    fn test_rename_note_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_target_exists_in_other_folder() {
        // Rename rejected even if the conflicting slug lives in a different folder.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("sub/b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.rename_note("missing", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_update_note_content_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "old content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.update_note("note", "note", "new content").unwrap();
        assert_eq!(meta.slug, "note");

        let saved = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert_eq!(saved, "new content");
    }

    #[test]
    fn test_update_note_rename_and_save() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "original content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let meta = cave
            .update_note("old", "new-name", "updated content")
            .unwrap();
        assert_eq!(meta.slug, "new-name");
        assert!(!dir.path().join("old.md").exists());

        let saved = std::fs::read_to_string(dir.path().join("new-name.md")).unwrap();
        assert_eq!(saved, "updated content");
    }

    #[test]
    fn test_update_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.update_note("ghost", "ghost", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_update_note_rename_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "a content").unwrap();
        std::fs::write(dir.path().join("b.md"), "b content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf());

        let err = cave.update_note("a", "b", "new content").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
        // Original file must be untouched
        let content = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(content, "a content");
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
        // Open cave after writing so index is populated.
        let cave = Cave::open(dir.path().to_path_buf());

        // listing still succeeds and includes the entry (slug is in the index from open-time scan)
        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "locked");

        // restore permissions so tempdir cleanup can delete the file
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}
