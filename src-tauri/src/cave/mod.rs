mod error;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use error::CaveError;

/// Metadata for a note in the cave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    /// Filename without extension (e.g., "my-note").
    pub slug: String,
    /// Display title — from frontmatter `title:` field, or slug as fallback.
    pub title: String,
    /// Relative path from cave root (e.g., "subfolder/my-note.md").
    pub relative_path: String,
}

/// Full note content returned when reading a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub meta: NoteMeta,
    pub content: String,
}

/// Validate a note name: must be non-empty, no path separators, no null bytes.
fn validate_name(name: &str) -> Result<(), CaveError> {
    if name.is_empty() {
        return Err(CaveError::InvalidName("name cannot be empty".to_string()));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(CaveError::InvalidName(
            "name cannot contain path separators".to_string(),
        ));
    }
    if name.starts_with('.') {
        return Err(CaveError::InvalidName(
            "name cannot start with a dot".to_string(),
        ));
    }
    Ok(())
}

/// Ensure the name has a .md extension, adding it if missing.
fn ensure_md_extension(name: &str) -> String {
    if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{name}.md")
    }
}

/// Extract a display title from the first line of a note's content.
/// Looks for a `# Heading` or YAML `title:` frontmatter field.
fn extract_title(content: &str, slug: &str) -> String {
    let trimmed = content.trim_start();

    // Check YAML frontmatter
    if let Some(after_prefix) = trimmed.strip_prefix("---") {
        if let Some(end) = after_prefix.find("---") {
            let frontmatter = &after_prefix[..end];
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some(title) = line.strip_prefix("title:") {
                    let title = title.trim().trim_matches('"').trim_matches('\'');
                    if !title.is_empty() {
                        return title.to_string();
                    }
                }
            }
        }
    }

    // Check for a # heading
    for line in trimmed.lines() {
        let line = line.trim();
        if let Some(heading) = line.strip_prefix("# ") {
            let heading = heading.trim();
            if !heading.is_empty() {
                return heading.to_string();
            }
        }
        // Skip empty lines at the start
        if !line.is_empty() {
            break;
        }
    }

    // Fallback to slug
    slug.to_string()
}

/// Create a new note in the cave. Returns the metadata of the created note.
pub fn create_note(cave_path: &Path, name: &str) -> Result<NoteMeta, CaveError> {
    validate_name(name)?;

    let filename = ensure_md_extension(name);
    let file_path = cave_path.join(&filename);

    if file_path.exists() {
        return Err(CaveError::AlreadyExists(filename));
    }

    let slug = filename.strip_suffix(".md").unwrap_or(&filename);
    let title = slug.to_string();
    let initial_content = format!("# {title}\n");

    std::fs::write(&file_path, &initial_content)?;

    Ok(NoteMeta {
        slug: slug.to_string(),
        title,
        relative_path: filename,
    })
}

/// List all notes in the cave (non-recursive, top-level .md files).
pub fn list_notes(cave_path: &Path) -> Result<Vec<NoteMeta>, CaveError> {
    let mut notes = Vec::new();

    for entry in std::fs::read_dir(cave_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let slug = filename
                        .strip_suffix(".md")
                        .unwrap_or(&filename)
                        .to_string();
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let title = extract_title(&content, &slug);

                    notes.push(NoteMeta {
                        slug,
                        title,
                        relative_path: filename,
                    });
                }
            }
        }
    }

    notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(notes)
}

/// Read a note's full content by slug or filename.
pub fn read_note(cave_path: &Path, name: &str) -> Result<Note, CaveError> {
    let filename = ensure_md_extension(name);
    let file_path = cave_path.join(&filename);

    if !file_path.exists() {
        return Err(CaveError::NotFound(filename));
    }

    let content = std::fs::read_to_string(&file_path)?;
    let slug = filename
        .strip_suffix(".md")
        .unwrap_or(&filename)
        .to_string();
    let title = extract_title(&content, &slug);

    Ok(Note {
        meta: NoteMeta {
            slug,
            title,
            relative_path: filename,
        },
        content,
    })
}

/// Save (overwrite) a note's content by slug or filename.
pub fn save_note(cave_path: &Path, name: &str, content: &str) -> Result<NoteMeta, CaveError> {
    let filename = ensure_md_extension(name);
    let file_path = cave_path.join(&filename);

    if !file_path.exists() {
        return Err(CaveError::NotFound(filename));
    }

    std::fs::write(&file_path, content)?;

    let slug = filename
        .strip_suffix(".md")
        .unwrap_or(&filename)
        .to_string();
    let title = extract_title(content, &slug);

    Ok(NoteMeta {
        slug,
        title,
        relative_path: filename,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("my-note").is_ok());
        assert!(validate_name("Note Title").is_ok());
        assert!(validate_name("2024-01-01").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("../escape").is_err());
        assert!(validate_name("sub/path").is_err());
        assert!(validate_name(".hidden").is_err());
    }

    #[test]
    fn test_ensure_md_extension() {
        assert_eq!(ensure_md_extension("note"), "note.md");
        assert_eq!(ensure_md_extension("note.md"), "note.md");
    }

    #[test]
    fn test_extract_title_heading() {
        assert_eq!(extract_title("# My Title\n\nContent", "slug"), "My Title");
    }

    #[test]
    fn test_extract_title_frontmatter() {
        let content = "---\ntitle: From Frontmatter\n---\n# Heading\nBody";
        assert_eq!(extract_title(content, "slug"), "From Frontmatter");
    }

    #[test]
    fn test_extract_title_fallback_to_slug() {
        assert_eq!(extract_title("Just some text", "my-slug"), "my-slug");
    }

    #[test]
    fn test_create_note() {
        let dir = tempfile::tempdir().unwrap();
        let meta = create_note(dir.path(), "my-note").unwrap();
        assert_eq!(meta.slug, "my-note");
        assert_eq!(meta.relative_path, "my-note.md");

        let content = std::fs::read_to_string(dir.path().join("my-note.md")).unwrap();
        assert!(content.contains("# my-note"));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        create_note(dir.path(), "test").unwrap();
        let err = create_note(dir.path(), "test").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_list_notes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(dir.path().join("beta.md"), "# Beta\n").unwrap();
        std::fs::write(dir.path().join("not-a-note.txt"), "ignore").unwrap();

        let notes = list_notes(dir.path()).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].title, "Alpha");
        assert_eq!(notes[1].title, "Beta");
    }

    #[test]
    fn test_read_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Test Note\nBody").unwrap();

        let note = read_note(dir.path(), "test").unwrap();
        assert_eq!(note.meta.title, "Test Note");
        assert!(note.content.contains("Body"));
    }

    #[test]
    fn test_read_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_note(dir.path(), "nonexistent").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();

        let meta = save_note(dir.path(), "test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.title, "New Title");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("New body"));
    }

    #[test]
    fn test_save_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = save_note(dir.path(), "missing", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }
}
