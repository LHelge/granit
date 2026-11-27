//! The last-open cave path, remembered across launches in a small JSON file
//! in the platform config directory (`<config dir>/se.lhelge.granit/`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name inside the app's config directory.
const FILE_NAME: &str = "last-cave.json";

#[derive(Debug, thiserror::Error)]
pub enum LastCaveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
struct Stored {
    path: PathBuf,
}

/// Where the last-open cave path is kept.
#[derive(Clone, Debug)]
pub struct LastCaveStore {
    file: PathBuf,
}

impl LastCaveStore {
    /// The store in the platform config directory, or `None` when the
    /// platform has none.
    pub fn in_config_dir() -> Option<Self> {
        dirs::config_dir().map(|dir| Self::at(dir.join(crate::host::APP_ID).join(FILE_NAME)))
    }

    /// A store backed by `file`.
    pub fn at(file: PathBuf) -> Self {
        Self { file }
    }

    /// The remembered path, or `None` when nothing is stored yet.
    pub fn load(&self) -> Result<Option<PathBuf>, LastCaveError> {
        let json = match std::fs::read(&self.file) {
            Ok(json) => json,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let stored: Stored = serde_json::from_slice(&json)?;
        Ok(Some(stored.path))
    }

    /// Remember `path` as the last-open cave.
    pub fn save(&self, path: &Path) -> Result<(), LastCaveError> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&Stored {
            path: path.to_path_buf(),
        })?;
        std::fs::write(&self.file, json)?;
        Ok(())
    }

    /// Forget the remembered path.
    pub fn clear(&self) -> Result<(), LastCaveError> {
        match std::fs::remove_file(&self.file) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &tempfile::TempDir) -> LastCaveStore {
        LastCaveStore::at(dir.path().join("nested/last-cave.json"))
    }

    #[test]
    fn nothing_is_stored_at_first() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(store(&dir).load().unwrap(), None);
    }

    #[test]
    fn a_saved_path_loads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        store.save(Path::new("/caves/mine")).unwrap();
        assert_eq!(store.load().unwrap(), Some(PathBuf::from("/caves/mine")));
    }

    #[test]
    fn clear_forgets_the_path_and_tolerates_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        store.clear().unwrap();
        store.save(Path::new("/caves/mine")).unwrap();
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn a_corrupt_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        store.save(Path::new("/caves/mine")).unwrap();
        std::fs::write(dir.path().join("nested/last-cave.json"), "not json").unwrap();
        assert!(matches!(store.load(), Err(LastCaveError::Json(_))));
    }
}
