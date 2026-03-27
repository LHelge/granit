mod error;
mod note;

use std::path::{Path, PathBuf};

pub use error::CaveError;
pub use note::{Note, NoteMeta};

use note::{ensure_md_extension, validate_name};

/// Resolve a validated filename from a user-supplied name.
/// Validates the name and ensures it has a `.md` extension.
fn resolve_filename(name: &str) -> Result<String, CaveError> {
    validate_name(name)?;
    Ok(ensure_md_extension(name))
}

/// A cave — an open directory of markdown notes.
pub struct Cave {
    path: PathBuf,
}

impl Cave {
    /// Open a cave at the given directory path.
    pub fn open(path: PathBuf) -> Self {
        Self { path }
    }

    /// The root directory of this cave.
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a new note in this cave. Returns the metadata of the created note.
    /// If the name is "untitled" and already exists, appends a numeric suffix (untitled-2, untitled-3, …).
    pub fn create_note(&self, name: &str) -> Result<NoteMeta, CaveError> {
        let base_filename = resolve_filename(name)?;
        let file_path = self.path.join(&base_filename);

        // For "untitled" notes, find a unique name automatically
        let filename = if file_path.exists() && name == "untitled" {
            let mut n = 2u32;
            loop {
                let candidate = format!("untitled-{n}.md");
                if !self.path.join(&candidate).exists() {
                    break candidate;
                }
                n += 1;
            }
        } else if file_path.exists() {
            return Err(CaveError::AlreadyExists(base_filename));
        } else {
            base_filename
        };

        let final_path = self.path.join(&filename);
        let slug = filename.strip_suffix(".md").unwrap_or(&filename);
        let title = slug.to_string();
        let initial_content = format!("# {title}\n");

        std::fs::write(&final_path, &initial_content)?;

        Ok(NoteMeta {
            slug: slug.to_string(),
            title,
            relative_path: filename,
        })
    }

    /// List all top-level .md notes in this cave.
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, CaveError> {
        let mut notes = Vec::new();

        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        let content = std::fs::read_to_string(&path).unwrap_or_default();
                        notes.push(NoteMeta::from_file(&filename, &content));
                    }
                }
            }
        }

        notes.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
        Ok(notes)
    }

    /// Read a note by slug or filename.
    pub fn read_note(&self, name: &str) -> Result<Note, CaveError> {
        let filename = resolve_filename(name)?;
        let file_path = self.path.join(&filename);

        if !file_path.exists() {
            return Err(CaveError::NotFound(filename));
        }

        let content = std::fs::read_to_string(&file_path)?;

        Ok(Note {
            meta: NoteMeta::from_file(&filename, &content),
            content,
        })
    }

    /// Save new content to an existing note.
    pub fn save_note(&self, name: &str, content: &str) -> Result<NoteMeta, CaveError> {
        let filename = resolve_filename(name)?;
        let file_path = self.path.join(&filename);

        if !file_path.exists() {
            return Err(CaveError::NotFound(filename));
        }

        std::fs::write(&file_path, content)?;
        Ok(NoteMeta::from_file(&filename, content))
    }

    /// Replace `old_text` with `new_text` in an existing note.
    /// Fails if `old_text` is not found in the note's content.
    #[allow(dead_code)]
    pub fn edit_note(
        &self,
        name: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<NoteMeta, CaveError> {
        let filename = resolve_filename(name)?;
        let file_path = self.path.join(&filename);

        if !file_path.exists() {
            return Err(CaveError::NotFound(filename));
        }

        let content = std::fs::read_to_string(&file_path)?;
        if !content.contains(old_text) {
            return Err(CaveError::EditNotFound);
        }
        let new_content = content.replacen(old_text, new_text, 1);

        std::fs::write(&file_path, &new_content)?;
        Ok(NoteMeta::from_file(&filename, &new_content))
    }

    /// Delete a note by slug or filename.
    pub fn delete_note(&self, name: &str) -> Result<(), CaveError> {
        let filename = resolve_filename(name)?;
        let file_path = self.path.join(&filename);

        if !file_path.exists() {
            return Err(CaveError::NotFound(filename));
        }

        std::fs::remove_file(&file_path)?;
        Ok(())
    }

    /// Rename an existing note. Fails if `new_name` already exists.
    pub fn rename_note(&self, old_name: &str, new_name: &str) -> Result<NoteMeta, CaveError> {
        let old_filename = resolve_filename(old_name)?;
        let new_filename = resolve_filename(new_name)?;

        // No-op if the name hasn't changed
        if old_filename == new_filename {
            return self.read_note(old_name).map(|n| n.meta);
        }

        let old_path = self.path.join(&old_filename);
        let new_path = self.path.join(&new_filename);

        if !old_path.exists() {
            return Err(CaveError::NotFound(old_filename));
        }
        if new_path.exists() {
            return Err(CaveError::AlreadyExists(new_filename));
        }

        std::fs::rename(&old_path, &new_path)?;

        let content = std::fs::read_to_string(&new_path)?;
        Ok(NoteMeta::from_file(&new_filename, &content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cave_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.create_note("hello").unwrap();
        assert_eq!(meta.slug, "hello");

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);

        let note = cave.read_note("hello").unwrap();
        assert!(note.content.contains("# hello"));

        cave.save_note("hello", "# Updated\nBody").unwrap();
        let note = cave.read_note("hello").unwrap();
        assert_eq!(note.meta.title, "Updated");

        cave.rename_note("hello", "world").unwrap();
        assert!(cave.read_note("hello").is_err());
        assert!(cave.read_note("world").is_ok());

        cave.delete_note("world").unwrap();
        assert!(cave.read_note("world").is_err());
    }

    #[test]
    fn test_create_note() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let meta = cave.create_note("my-note").unwrap();
        assert_eq!(meta.slug, "my-note");
        assert_eq!(meta.relative_path, "my-note.md");

        let content = std::fs::read_to_string(dir.path().join("my-note.md")).unwrap();
        assert!(content.contains("# my-note"));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        cave.create_note("test").unwrap();
        let err = cave.create_note("test").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_untitled_auto_numbering() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let m1 = cave.create_note("untitled").unwrap();
        assert_eq!(m1.slug, "untitled");

        let m2 = cave.create_note("untitled").unwrap();
        assert_eq!(m2.slug, "untitled-2");

        let m3 = cave.create_note("untitled").unwrap();
        assert_eq!(m3.slug, "untitled-3");
    }

    #[test]
    fn test_list_notes() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(dir.path().join("beta.md"), "# Beta\n").unwrap();
        std::fs::write(dir.path().join("not-a-note.txt"), "ignore").unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].title, "Alpha");
        assert_eq!(notes[1].title, "Beta");
    }

    #[test]
    fn test_read_note() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("test.md"), "# Test Note\nBody").unwrap();

        let note = cave.read_note("test").unwrap();
        assert_eq!(note.meta.title, "Test Note");
        assert!(note.content.contains("Body"));
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
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();

        let meta = cave.save_note("test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.title, "New Title");

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
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("test.md"), "# Hello\nold text here").unwrap();

        let meta = cave.edit_note("test", "old text", "new text").unwrap();
        assert_eq!(meta.title, "Hello");

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
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("test.md"), "# Hello\nsome content").unwrap();

        let err = cave
            .edit_note("test", "nonexistent text", "replacement")
            .unwrap_err();
        assert!(matches!(err, CaveError::EditNotFound));
    }

    #[test]
    fn test_edit_note_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("test.md"), "AAA BBB AAA").unwrap();

        cave.edit_note("test", "AAA", "CCC").unwrap();
        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert_eq!(content, "CCC BBB AAA");
    }

    #[test]
    fn test_delete_note() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("doomed.md"), "# Bye\n").unwrap();
        assert!(dir.path().join("doomed.md").exists());

        cave.delete_note("doomed").unwrap();
        assert!(!dir.path().join("doomed.md").exists());
    }

    #[test]
    fn test_delete_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.delete_note("ghost").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.delete_note("../escape").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_rename_note() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("old.md"), "# Old Note\n").unwrap();

        let meta = cave.rename_note("old", "new-name").unwrap();
        assert_eq!(meta.slug, "new-name");
        assert_eq!(meta.relative_path, "new-name.md");
        assert!(!dir.path().join("old.md").exists());
        assert!(dir.path().join("new-name.md").exists());
    }

    #[test]
    fn test_rename_note_noop() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("same.md"), "# Same\n").unwrap();

        let meta = cave.rename_note("same", "same").unwrap();
        assert_eq!(meta.slug, "same");
        assert!(dir.path().join("same.md").exists());
    }

    #[test]
    fn test_rename_note_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# B\n").unwrap();

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf());

        let err = cave.rename_note("missing", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }
}
