use serde::{Deserialize, Serialize};

use super::CaveError;

/// Metadata for a note in the cave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    /// Filename without extension (e.g., "my-note").
    pub slug: String,
    /// Relative path from cave root (e.g., "subfolder/my-note.md").
    pub relative_path: String,
}

impl NoteMeta {
    /// Build metadata from a filename.
    pub(crate) fn from_file(filename: &str) -> Self {
        let slug = filename.strip_suffix(".md").unwrap_or(filename).to_string();
        Self {
            slug,
            relative_path: filename.to_string(),
        }
    }
}

/// Full note content returned when reading a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub meta: NoteMeta,
    pub content: String,
}

/// Validate a note name: must be non-empty, no path separators, no null bytes.
pub(crate) fn validate_name(name: &str) -> Result<(), CaveError> {
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
pub(crate) fn ensure_md_extension(name: &str) -> String {
    if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{name}.md")
    }
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
}
