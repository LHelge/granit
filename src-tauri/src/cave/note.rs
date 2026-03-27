use serde::{Deserialize, Serialize};

use super::CaveError;

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

impl NoteMeta {
    /// Build metadata from a filename and the note's content.
    pub(crate) fn from_file(filename: &str, content: &str) -> Self {
        let slug = filename.strip_suffix(".md").unwrap_or(filename).to_string();
        let title = extract_title(content, &slug);
        Self {
            slug,
            title,
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

/// Extract a display title from the first line of a note's content.
/// Looks for a `# Heading` or YAML `title:` frontmatter field.
pub(crate) fn extract_title(content: &str, slug: &str) -> String {
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
}
