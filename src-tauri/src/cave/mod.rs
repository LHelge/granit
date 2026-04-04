mod error;
mod note;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use error::CaveError;
pub use note::{Note, NoteMeta};

use note::{
    ensure_md_extension, note_meta_from_relative_path, note_meta_with_icon, validate_folder_path,
    validate_name,
};

/// A cave — an open directory of markdown notes.
#[derive(Debug)]
pub struct Cave {
    path: PathBuf,
    /// In-memory index: slug → absolute path. Populated at open and kept in
    /// sync by create / delete / rename / update operations.
    /// Slug uniqueness is enforced globally across all subdirectories.
    notes: HashMap<String, PathBuf>,
    /// Slug of the note currently open in the editor, if any.
    active_slug: Option<String>,
}

impl Cave {
    /// Open a cave at the given directory path. Eagerly scans recursively for
    /// `.md` files to populate the in-memory notes index.
    ///
    /// Returns an error if two files share the same slug (filename without `.md`).
    pub fn open(path: PathBuf) -> Result<Self, CaveError> {
        let notes = Self::scan_recursive(&path, &path)?;
        Ok(Self {
            path,
            notes,
            active_slug: None,
        })
    }

    /// Recursively scan `dir` for `.md` files and return a slug → absolute-path map.
    ///
    /// Subdirectories starting with `.` (hidden) are skipped, as is `.granit/`.
    /// Returns an error if two files share the same slug.
    fn scan_recursive(cave_root: &Path, dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
        let mut notes: HashMap<String, PathBuf> = HashMap::new();
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
                let sub = Self::scan_recursive(cave_root, &p)?;
                for (slug, abs_path) in sub {
                    match notes.entry(slug) {
                        std::collections::hash_map::Entry::Occupied(e) => {
                            let existing_rel = e
                                .get()
                                .strip_prefix(cave_root)
                                .unwrap_or(e.get())
                                .to_string_lossy()
                                .into_owned();
                            let new_rel = abs_path
                                .strip_prefix(cave_root)
                                .unwrap_or(&abs_path)
                                .to_string_lossy()
                                .into_owned();
                            return Err(CaveError::DuplicateSlug {
                                slug: e.key().clone(),
                                paths: vec![existing_rel, new_rel],
                            });
                        }
                        std::collections::hash_map::Entry::Vacant(v) => {
                            v.insert(abs_path);
                        }
                    }
                }
            } else if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext == "md" {
                        let slug = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        match notes.entry(slug) {
                            std::collections::hash_map::Entry::Occupied(e) => {
                                let existing_rel = e
                                    .get()
                                    .strip_prefix(cave_root)
                                    .unwrap_or(e.get())
                                    .to_string_lossy()
                                    .into_owned();
                                let new_rel = p
                                    .strip_prefix(cave_root)
                                    .unwrap_or(&p)
                                    .to_string_lossy()
                                    .into_owned();
                                return Err(CaveError::DuplicateSlug {
                                    slug: e.key().clone(),
                                    paths: vec![existing_rel, new_rel],
                                });
                            }
                            std::collections::hash_map::Entry::Vacant(v) => {
                                v.insert(p);
                            }
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

    /// Resolve a slug case-insensitively, returning the canonical stored slug.
    ///
    /// Returns `CaveError::NotFound` if no note matches.
    pub fn resolve_slug(&self, slug: &str) -> Result<String, CaveError> {
        self.lookup_slug(slug)
            .map(String::from)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))
    }

    /// The root directory of this cave.
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Set the slug of the note currently open in the editor.
    pub fn set_active_slug(&mut self, slug: Option<String>) {
        self.active_slug = slug;
    }

    /// Get the slug of the note currently open in the editor.
    pub fn active_slug(&self) -> Option<&str> {
        self.active_slug.as_deref()
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

    /// Open or create today's daily note in the given folder.
    ///
    /// `folder` is a relative path from the cave root (e.g. `"Daily"` or `"Notes/Daily"`).
    /// The folder is created if it does not yet exist.
    /// If the note for today already exists it is read and returned without modification.
    /// The note slug is today's date in `YYYY-MM-DD` format.
    pub fn open_daily_note(&mut self, folder: &str) -> Result<Note, CaveError> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let folder_path = Path::new(folder);

        // Ensure the daily folder exists.
        let abs_folder = self.path.join(folder_path);
        if !abs_folder.is_dir() {
            validate_folder_path(folder_path)?;
            std::fs::create_dir_all(&abs_folder)?;
        }

        // Create the note if it doesn't exist yet, setting the calendar icon.
        if !self.notes.contains_key(today.as_str()) {
            self.create_note(&today, Some(folder_path))?;
            let abs_path = self.notes[today.as_str()].clone();
            let raw = std::fs::read_to_string(&abs_path)?;
            let updated = crate::markdown::rebuild_with_frontmatter(
                &raw,
                "",
                None,
                Some("Calendar".to_string()),
            );
            std::fs::write(&abs_path, updated)?;
        }

        self.read_note(&today)
    }

    /// Create a subdirectory within the cave.
    ///
    /// `path` is a relative path from the cave root (e.g. `Path::new("notes")` or `Path::new("projects/2026")`).
    #[allow(dead_code)]
    pub fn create_folder(&mut self, path: &Path) -> Result<(), CaveError> {
        validate_folder_path(path)?;
        let abs = self.path.join(path);
        if abs.exists() {
            return Err(CaveError::AlreadyExists(
                path.to_string_lossy().into_owned(),
            ));
        }
        std::fs::create_dir_all(&abs)?;
        Ok(())
    }

    /// Delete a folder and all notes within it.
    ///
    /// `path` is a relative path from the cave root (e.g. `Path::new("notes")` or `Path::new("projects/2026")`).
    /// All notes indexed under this folder are removed from the cave index before deletion.
    pub fn delete_folder(&mut self, path: &Path) -> Result<(), CaveError> {
        validate_folder_path(path)?;
        let abs = self.path.join(path);
        if !abs.exists() {
            return Err(CaveError::NotFound(path.to_string_lossy().into_owned()));
        }
        // Filesystem first — only update the index after the delete succeeds.
        std::fs::remove_dir_all(&abs)?;
        self.notes.retain(|_, note_abs| !note_abs.starts_with(&abs));
        Ok(())
    }

    /// List all `.md` notes in this cave (recursively), sorted by slug.
    ///
    /// Each note's frontmatter is read to populate the `icon` field.
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, CaveError> {
        let mut notes: Vec<NoteMeta> = self
            .notes
            .values()
            .map(|abs| note_meta_with_icon(&self.relative_path(abs), abs))
            .collect();
        notes.sort_by_key(|n| n.slug.to_lowercase());
        Ok(notes)
    }

    /// List all subdirectory relative paths in this cave (recursively), sorted.
    ///
    /// Hidden directories (starting with `.`) and `.granit/` are excluded.
    /// Returns paths like `"projects"`, `"projects/2026"`.
    pub fn list_folders(&self) -> Result<Vec<String>, CaveError> {
        let mut folders = Vec::new();
        Self::collect_folders(&self.path, &self.path, &mut folders)?;
        folders.sort_by_key(|f| f.to_lowercase());
        Ok(folders)
    }

    /// Recursively collect subdirectory relative paths.
    fn collect_folders(
        cave_root: &Path,
        dir: &Path,
        out: &mut Vec<String>,
    ) -> Result<(), CaveError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == ".granit" {
                continue;
            }
            let rel = p
                .strip_prefix(cave_root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            out.push(rel);
            Self::collect_folders(cave_root, &p, out)?;
        }
        Ok(())
    }

    /// Read a note by slug.
    pub fn read_note(&self, slug: &str) -> Result<Note, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let body = crate::markdown::strip_frontmatter(&raw).to_string();
        let rel = self.relative_path(abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&raw);
        Ok(Note {
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
    pub fn save_note(&self, slug: &str, content: &str) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let existing_raw = std::fs::read_to_string(abs_path)?;
        let updated = crate::markdown::rebuild_with_frontmatter(&existing_raw, content, None, None);
        std::fs::write(abs_path, updated.as_str())?;
        let rel = self.relative_path(abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        Ok(meta)
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

        let raw = std::fs::read_to_string(&abs_path)?;
        let body = crate::markdown::strip_frontmatter(&raw);
        if !body.contains(old_text) {
            return Err(CaveError::EditNotFound);
        }
        let new_body = body.replacen(old_text, new_text, 1);
        let new_content = crate::markdown::rebuild_with_frontmatter(&raw, &new_body, None, None);
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

    /// Move a note to a different folder within the cave.
    ///
    /// `destination` is a relative path from the cave root (e.g. `Path::new("projects")`).
    /// Passing `None` moves the note to the cave root.
    pub fn move_note(
        &mut self,
        slug: &str,
        destination: Option<&Path>,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let old_abs = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let dest_dir = if let Some(d) = destination {
            validate_folder_path(d)?;
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
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
            return Ok(note_meta_with_icon(&rel, &old_abs));
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
        Ok(note_meta_with_icon(&rel, &new_abs))
    }

    /// Move a folder under a new parent within the cave.
    ///
    /// `source` is the relative path of the folder to move.
    /// `destination` is the relative path of the new parent directory (`None` = cave root).
    pub fn move_folder(
        &mut self,
        source: &Path,
        destination: Option<&Path>,
    ) -> Result<(), CaveError> {
        validate_folder_path(source)?;
        if let Some(d) = destination {
            validate_folder_path(d)?;
        }

        let src_abs = self.path.join(source);
        if !src_abs.is_dir() {
            return Err(CaveError::NotFound(source.to_string_lossy().into_owned()));
        }

        let folder_name = source
            .file_name()
            .ok_or_else(|| CaveError::InvalidName("folder has no name".into()))?;

        let dest_parent = if let Some(d) = destination {
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
            abs
        } else {
            self.path.clone()
        };

        let new_abs = dest_parent.join(folder_name);

        // No-op.
        if src_abs == new_abs {
            return Ok(());
        }

        // Prevent moving into itself or a subdirectory of itself.
        if new_abs.starts_with(&src_abs) {
            return Err(CaveError::InvalidName(
                "cannot move a folder into itself".into(),
            ));
        }

        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&src_abs, &new_abs)?;

        self.update_child_paths(&src_abs, &new_abs);

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
        Ok(note_meta_with_icon(&rel, &new_abs))
    }

    /// Update a note's filename, content, and optionally tags and icon in one operation.
    ///
    /// If `old_slug` and `new_name` differ the file is renamed first (same directory),
    /// then the new content is written. If `tags` is `Some`, the frontmatter tags
    /// are replaced. If `icon` is `Some`, the icon is set (`Some("")` clears it).
    pub fn update_note(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
    ) -> Result<NoteMeta, CaveError> {
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
        let updated = crate::markdown::rebuild_with_frontmatter(&existing_raw, content, tags, icon);
        if let Err(e) = std::fs::write(&final_abs, updated.as_str()) {
            // Rollback the rename so index stays consistent with filesystem.
            if renamed {
                let _ = std::fs::rename(&final_abs, &old_abs);
            }
            return Err(e.into());
        }

        // Update index only after all filesystem operations succeed.
        if renamed {
            self.notes.remove(old_slug);
            self.notes.insert(new_name.to_string(), final_abs.clone());
        }

        let rel = self.relative_path(&final_abs);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        Ok(meta)
    }

    /// Rename a folder in-place (same parent directory, new name).
    ///
    /// `source` is the relative path of the folder to rename.
    /// `new_name` is the new name for the folder (just the final component, not a path).
    /// Update indexed note paths after a folder is moved or renamed.
    /// Replaces `old_prefix` with `new_prefix` for every note under the old location.
    fn update_child_paths(&mut self, old_prefix: &Path, new_prefix: &Path) {
        let updates: Vec<(String, PathBuf)> = self
            .notes
            .iter()
            .filter(|(_, abs)| abs.starts_with(old_prefix))
            .map(|(slug, abs)| {
                let suffix = abs.strip_prefix(old_prefix).unwrap();
                (slug.clone(), new_prefix.join(suffix))
            })
            .collect();
        for (slug, new_path) in updates {
            self.notes.insert(slug, new_path);
        }
    }

    pub fn rename_folder(&mut self, source: &Path, new_name: &str) -> Result<(), CaveError> {
        validate_folder_path(source)?;
        validate_name(new_name)?;

        let src_abs = self.path.join(source);
        if !src_abs.is_dir() {
            return Err(CaveError::NotFound(source.to_string_lossy().into_owned()));
        }

        let parent = src_abs.parent().unwrap_or(&self.path);
        let new_abs = parent.join(new_name);

        // No-op if nothing changed.
        if src_abs == new_abs {
            return Ok(());
        }

        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&src_abs, &new_abs)?;

        self.update_child_paths(&src_abs, &new_abs);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cave_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_note("hello", None).unwrap();
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

        let meta = cave.create_note("my-note", None).unwrap();
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
            .create_note("nested", Some(Path::new("notes")))
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
            .create_note("note", Some(Path::new("nonexistent")))
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_note("test", None).unwrap();
        let err = cave.create_note("test", None).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_note_duplicate_slug_across_folders() {
        // Two notes with the same filename in different folders should be rejected.
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        cave.create_note("foo", None).unwrap();
        let err = cave.create_note("foo", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_untitled_auto_numbering() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

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
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_folder(Path::new("projects")).unwrap();
        assert!(dir.path().join("projects").is_dir());
    }

    #[test]
    fn test_create_folder_nested() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_folder(Path::new("a/b/c")).unwrap();
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn test_create_folder_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        std::fs::create_dir(dir.path().join("existing")).unwrap();
        let err = cave.create_folder(Path::new("existing")).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_delete_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        std::fs::write(dir.path().join("root.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(cave.notes.contains_key("note"));
        cave.delete_folder(Path::new("sub")).unwrap();
        assert!(
            !cave.notes.contains_key("note"),
            "note should be removed from index"
        );
        assert!(cave.notes.contains_key("root"), "root note should remain");
        assert!(
            !dir.path().join("sub").exists(),
            "folder should be deleted from disk"
        );
    }

    #[test]
    fn test_delete_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let err = cave.delete_folder(Path::new("ghost")).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_folder_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let err = cave.delete_folder(Path::new("../escape")).unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
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
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.save_note("test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("New body"));
    }

    #[test]
    fn test_save_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("missing", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_save_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("../escape", "content").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_edit_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nold text here").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.edit_note("test", "old text", "new text").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("new text here"));
        assert!(!content.contains("old text"));
    }

    #[test]
    fn test_edit_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.edit_note("missing", "old", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_edit_note_text_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nsome content").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .edit_note("test", "nonexistent text", "replacement")
            .unwrap_err();
        assert!(matches!(err, CaveError::EditNotFound));
    }

    #[test]
    fn test_edit_note_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "AAA BBB AAA").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

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

    // ── move_note tests ────────────────────────────────────────────

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
        cave.create_note("note", None).unwrap();
        // Place a file at the destination path that isn't in the index.
        std::fs::write(dir.path().join("sub/note.md"), "conflict").unwrap();

        let err = cave.move_note("note", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_open_cave_rejects_duplicate_slugs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("dup.md"), "root").unwrap();
        std::fs::write(dir.path().join("sub/dup.md"), "sub").unwrap();

        let err = Cave::open(dir.path().to_path_buf()).unwrap_err();
        assert!(
            matches!(err, CaveError::DuplicateSlug { ref slug, .. } if slug == "dup"),
            "expected DuplicateSlug error, got: {err:?}"
        );
    }

    // ── move_folder tests ──────────────────────────────────────────

    #[test]
    fn test_move_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("dest")).unwrap();
        std::fs::write(dir.path().join("src/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("src"), Some(Path::new("dest")))
            .unwrap();
        assert!(dir.path().join("dest/src/note.md").exists());
        assert!(!dir.path().join("src").exists());
        // Index should be updated.
        let note = cave.read_note("note").unwrap();
        assert_eq!(note.meta.relative_path, "dest/src/note.md");
    }

    #[test]
    fn test_move_folder_to_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parent/child")).unwrap();
        std::fs::write(dir.path().join("parent/child/note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("parent/child"), None).unwrap();
        assert!(dir.path().join("child/note.md").exists());
        let note = cave.read_note("note").unwrap();
        assert_eq!(note.meta.relative_path, "child/note.md");
    }

    #[test]
    fn test_move_folder_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parent/child")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("parent/child"), Some(Path::new("parent")))
            .unwrap();
        assert!(dir.path().join("parent/child").exists());
    }

    #[test]
    fn test_move_folder_into_itself_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("a/b")))
            .unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_move_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.move_folder(Path::new("ghost"), None).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_folder_dest_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::create_dir(dir.path().join("b/a")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("b")))
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

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
        // Rename rejected even if the conflicting slug lives in a different folder.
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
    fn test_update_note_content_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "old content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("note", "note", "new content", None, None)
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
            .update_note("old", "new-name", "updated content", None, None)
            .unwrap();
        assert_eq!(meta.slug, "new-name");
        assert!(!dir.path().join("old.md").exists());

        let saved = std::fs::read_to_string(dir.path().join("new-name.md")).unwrap();
        assert_eq!(saved, "updated content");
    }

    #[test]
    fn test_update_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .update_note("ghost", "ghost", "content", None, None)
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
            .update_note("a", "b", "new content", None, None)
            .unwrap_err();
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
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        // listing still succeeds and includes the entry (slug is in the index from open-time scan)
        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "locked");

        // restore permissions so tempdir cleanup can delete the file
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn test_list_folders_empty_cave() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert!(folders.is_empty());
    }

    #[test]
    fn test_list_folders_includes_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["empty"]);
    }

    #[test]
    fn test_list_folders_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::create_dir(dir.path().join("c")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["a", "a/b", "c"]);
    }

    #[test]
    fn test_list_folders_skips_hidden_and_granit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
        std::fs::create_dir(dir.path().join("visible")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["visible"]);
    }

    #[test]
    fn test_open_daily_note_creates_folder_and_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave.open_daily_note("Daily").unwrap();

        assert_eq!(note.meta.slug, today);
        assert_eq!(note.meta.relative_path, format!("Daily/{today}.md"));
        assert!(dir.path().join("Daily").is_dir());
        assert!(dir.path().join(format!("Daily/{today}.md")).exists());
        assert_eq!(note.meta.icon.as_deref(), Some("Calendar"));
    }

    #[test]
    fn test_open_daily_note_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note1 = cave.open_daily_note("Daily").unwrap();
        let note2 = cave.open_daily_note("Daily").unwrap();

        // Second call must not create a duplicate or modify the slug
        assert_eq!(note1.meta.slug, note2.meta.slug);
        let daily_notes: Vec<_> = std::fs::read_dir(dir.path().join("Daily"))
            .unwrap()
            .collect();
        assert_eq!(daily_notes.len(), 1, "should only have one daily note file");
    }

    #[test]
    fn test_open_daily_note_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Journal")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave.open_daily_note("Journal").unwrap();
        assert_eq!(note.meta.slug, today);
        assert!(dir.path().join(format!("Journal/{today}.md")).exists());
    }
}
