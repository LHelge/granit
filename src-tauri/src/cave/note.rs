use std::path::{Component, Path};

use super::CaveError;

pub use granit_types::{Note, NoteMeta};

/// Build note metadata from a relative path (e.g. `Path::new("folder/note.md")`).
///
/// `NoteMeta.relative_path` is always stored with forward slashes so it is
/// consistent across platforms when sent over IPC.
pub(crate) fn note_meta_from_relative_path(relative_path: &Path) -> NoteMeta {
    let slug = relative_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Collect Normal components only, joined with '/' for cross-platform IPC.
    let path_str = relative_path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    NoteMeta {
        slug,
        relative_path: path_str,
    }
}

/// Validate a bare note filename (no directory components).
/// Must be non-empty, no path separators, no null bytes, not starting with a dot.
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

/// Validate a relative folder path using `Path::components()`.
///
/// Rejects traversal (`..`), current-dir (`.`), root, and hidden components.
#[allow(dead_code)]
pub(crate) fn validate_folder_path(path: &Path) -> Result<(), CaveError> {
    let mut has_components = false;
    for component in path.components() {
        match component {
            Component::Normal(s) => {
                has_components = true;
                let s = s.to_string_lossy();
                if s.starts_with('.') {
                    return Err(CaveError::InvalidName(format!(
                        "path component cannot start with a dot: {s:?}"
                    )));
                }
                if s.contains('\0') {
                    return Err(CaveError::InvalidName(format!(
                        "invalid characters in path component: {s:?}"
                    )));
                }
            }
            _ => {
                return Err(CaveError::InvalidName(format!(
                    "invalid path component: {component:?}"
                )));
            }
        }
    }
    if !has_components {
        return Err(CaveError::InvalidName(
            "folder path cannot be empty".to_string(),
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

    #[test]
    fn test_note_meta_from_relative_path_root() {
        let meta = note_meta_from_relative_path(Path::new("foo.md"));
        assert_eq!(meta.slug, "foo");
        assert_eq!(meta.relative_path, "foo.md");
    }

    #[test]
    fn test_note_meta_from_relative_path_nested() {
        let meta = note_meta_from_relative_path(Path::new("folder/sub/bar.md"));
        assert_eq!(meta.slug, "bar");
        assert_eq!(meta.relative_path, "folder/sub/bar.md");
    }

    #[test]
    fn test_validate_folder_path_valid() {
        assert!(validate_folder_path(Path::new("notes")).is_ok());
        assert!(validate_folder_path(Path::new("a/b/c")).is_ok());
        assert!(validate_folder_path(Path::new("My Folder")).is_ok());
    }

    #[test]
    fn test_validate_folder_path_invalid() {
        assert!(validate_folder_path(Path::new("")).is_err());
        assert!(validate_folder_path(Path::new("..")).is_err());
        assert!(validate_folder_path(Path::new("a/../b")).is_err());
        assert!(validate_folder_path(Path::new(".hidden")).is_err());
    }
}
