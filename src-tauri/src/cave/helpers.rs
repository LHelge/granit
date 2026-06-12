use std::path::{Component, Path};

use super::CaveError;

pub use granit_types::{Document, DocumentMeta, RenderedDocument};

/// Write `bytes` to `path` atomically.
///
/// Delegates to [`atomic_write_file`], which creates a hidden sibling
/// temporary file (`.<name>.XXXXXX`), fsyncs it, and renames over the
/// target. On Linux with the `unnamed-tmpfile` feature the temp file is
/// `O_TMPFILE`, so a crash before commit leaves no leftover files.
/// On POSIX the rename is atomic; on Windows the crate uses the
/// platform's atomic replace primitive.
///
/// Using the hidden-dotfile tmp convention means a crashed commit also
/// cannot be picked up by the cave scanner (which skips dotfiles and
/// files without a `.md` extension).
pub(crate) fn write_atomic(path: &Path, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = atomic_write_file::AtomicWriteFile::open(path)?;
    file.write_all(bytes.as_ref())?;
    file.commit()?;
    Ok(())
}

/// Create `path` with `bytes`, failing if it already exists.
///
/// Uses `OpenOptions::create_new(true)` so the existence check and claim are
/// atomic at the filesystem level, closing the TOCTOU window that a separate
/// `exists()` check would leave open. Contents are fsynced before returning.
pub(crate) fn write_new(path: &Path, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes.as_ref())?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = result {
        // Best effort: remove the partial file we just created.
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(e);
    }
    Ok(())
}

/// Build note metadata including frontmatter-derived fields from a file on disk.
///
/// Reads frontmatter from `abs_path` to populate fields like `icon` and
/// `favorite`. Falls back to `None` if the file cannot be read or the field is
/// absent.
pub(crate) fn note_meta_with_frontmatter(
    relative_path: &Path,
    abs_path: &std::path::Path,
) -> DocumentMeta {
    let mut meta = note_meta_from_relative_path(relative_path);
    if let Ok(raw) = std::fs::read_to_string(abs_path) {
        let md = crate::markdown::Markdown::new(&raw);
        meta.icon = md.icon();
        meta.favorite = md.favorite();
    }
    meta
}

/// Build document metadata from a relative path (e.g. `Path::new("folder/note.md")`).
///
/// `DocumentMeta.relative_path` is always stored with forward slashes so it is
/// consistent across platforms when sent over IPC.
pub(crate) fn note_meta_from_relative_path(relative_path: &Path) -> DocumentMeta {
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
    DocumentMeta {
        slug,
        relative_path: path_str,
        icon: None,
        favorite: None,
    }
}

/// Build template metadata including the icon from a file on disk.
///
/// `favorite` is always `None` for templates.
pub(crate) fn template_meta_with_icon(abs_path: &std::path::Path) -> DocumentMeta {
    let mut meta = template_meta_from_path(abs_path);
    if let Ok(raw) = std::fs::read_to_string(abs_path) {
        meta.icon = crate::markdown::Markdown::new(&raw).icon();
    }
    meta
}

/// Build template metadata from a template file path.
///
/// `favorite` is always `None` for templates.
pub(crate) fn template_meta_from_path(path: &Path) -> DocumentMeta {
    let slug = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let relative_path = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    DocumentMeta {
        slug,
        relative_path,
        icon: None,
        favorite: None,
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

/// Strip any trailing `.md` extension(s) from a user-supplied note/template name.
///
/// The slug is always the filename stem, so a name like `"foo.md"` must become
/// the slug `"foo"` (file `foo.md`). Without this, the index would key the note
/// under `"foo.md"` while every rescan derives `"foo"` from the file stem — and
/// a rename to `"bar.md"` would rewrite inbound wiki-links to the bogus
/// `[[bar.md]]` on disk, permanently breaking them after the next restart.
pub(crate) fn normalize_note_name(name: &str) -> &str {
    let mut name = name;
    while let Some(stripped) = name.strip_suffix(".md") {
        name = stripped;
    }
    name
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
        assert!(meta.favorite.is_none());
    }

    #[test]
    fn test_note_meta_from_relative_path_nested() {
        let meta = note_meta_from_relative_path(Path::new("folder/sub/bar.md"));
        assert_eq!(meta.slug, "bar");
        assert_eq!(meta.relative_path, "folder/sub/bar.md");
    }

    #[test]
    fn test_template_meta_from_path_has_no_favorite() {
        let meta = template_meta_from_path(Path::new("daily.md"));
        assert_eq!(meta.slug, "daily");
        assert!(meta.favorite.is_none());
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

    #[test]
    fn test_write_atomic_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        write_atomic(&target, b"hello").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
    }

    #[test]
    fn test_write_atomic_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        std::fs::write(&target, "old").unwrap();
        write_atomic(&target, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }

    #[test]
    fn test_write_atomic_leaves_no_tmp_files_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        write_atomic(&target, b"payload").unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        // Only the committed file should remain; no sibling temp/dotfiles.
        assert_eq!(entries, vec!["note.md".to_string()]);
    }
}
