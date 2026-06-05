use super::helpers::{validate_folder_path, validate_name};
use super::{Cave, CaveError};
use std::path::Path;

impl Cave {
    /// Create a subdirectory within the cave.
    ///
    /// `path` is a relative path from the cave root (e.g. `Path::new("notes")` or `Path::new("projects/2026")`).
    pub fn create_folder(&mut self, path: &Path) -> Result<(), CaveError> {
        validate_folder_path(path)?;
        let abs = self.path.join(path);
        self.check_containment(&abs)?;
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
        self.check_containment(&abs)?;
        // Filesystem first — only update the index after the delete succeeds.
        std::fs::remove_dir_all(&abs)?;
        self.notes.retain(|_, note_abs| !note_abs.starts_with(&abs));
        self.rebuild_link_indexes();
        Ok(())
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
        self.check_containment(&src_abs)?;

        let folder_name = source
            .file_name()
            .ok_or_else(|| CaveError::InvalidName("folder has no name".into()))?;

        let dest_parent = if let Some(d) = destination {
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
            self.check_containment(&abs)?;
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

    /// Rename a folder in-place (same parent directory, new name).
    ///
    /// `source` is the relative path of the folder to rename.
    /// `new_name` is the new name for the folder (just the final component, not a path).
    pub fn rename_folder(&mut self, source: &Path, new_name: &str) -> Result<(), CaveError> {
        validate_folder_path(source)?;
        validate_name(new_name)?;

        let src_abs = self.path.join(source);
        if !src_abs.is_dir() {
            return Err(CaveError::NotFound(source.to_string_lossy().into_owned()));
        }
        self.check_containment(&src_abs)?;

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

    /// Update indexed note paths after a folder is moved or renamed.
    /// Replaces `old_prefix` with `new_prefix` for every note under the old location.
    pub(crate) fn update_child_paths(&mut self, old_prefix: &Path, new_prefix: &Path) {
        let updates: Vec<(String, std::path::PathBuf)> = self
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
}

#[cfg(test)]
mod tests {
    use crate::cave::{Cave, CaveError};
    use std::path::Path;

    #[test]
    fn test_create_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_folder(Path::new("new-folder")).unwrap();
        assert!(dir.path().join("new-folder").is_dir());
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

        cave.create_folder(Path::new("dup")).unwrap();
        let err = cave.create_folder(Path::new("dup")).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    // ── delete_folder ──────────────────────────────────────────────

    #[test]
    fn test_delete_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("trash")).unwrap();
        std::fs::write(dir.path().join("trash/note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.delete_folder(Path::new("trash")).unwrap();
        assert!(!dir.path().join("trash").exists());
    }

    #[test]
    fn test_delete_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_folder(Path::new("nope")).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_folder_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_folder(Path::new("../sneaky")).unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    // ── list_folders ───────────────────────────────────────────────

    #[test]
    fn test_list_folders_empty_cave() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(cave.list_folders().unwrap().is_empty());
    }

    #[test]
    fn test_list_folders_includes_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let folders = cave.list_folders().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0], "empty");
    }

    #[test]
    fn test_list_folders_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let folders = cave.list_folders().unwrap();
        assert!(folders.contains(&"a".to_string()));
        assert!(folders.contains(&"a/b".to_string()));
    }

    #[test]
    fn test_list_folders_skips_hidden_and_granit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::create_dir(dir.path().join(".granit")).unwrap();
        std::fs::create_dir(dir.path().join("visible")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["visible"]);
    }

    // ── move_folder ────────────────────────────────────────────────

    #[test]
    fn test_move_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("dest")).unwrap();
        std::fs::write(dir.path().join("src/note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("src"), Some(Path::new("dest")))
            .unwrap();
        assert!(!dir.path().join("src").exists());
        assert!(dir.path().join("dest/src/note.md").exists());
    }

    #[test]
    fn test_move_folder_to_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parent/child")).unwrap();
        std::fs::write(dir.path().join("parent/child/note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("parent/child"), None).unwrap();
        assert!(dir.path().join("child/note.md").exists());
        assert!(!dir.path().join("parent/child").exists());
    }

    #[test]
    fn test_move_folder_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("folder")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("folder"), None).unwrap();
        assert!(dir.path().join("folder").exists());
    }

    #[test]
    fn test_move_folder_into_itself_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("a")))
            .unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_move_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("nope"), Some(Path::new("dest")))
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_folder_dest_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("dest")).unwrap();
        std::fs::create_dir(dir.path().join("dest/a")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("dest")))
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }
}
