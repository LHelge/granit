//! User-authored agent documents stored under `.granit/agent/`.
//!
//! These files are edited as raw text (frontmatter included) and are never
//! round-tripped through the typed frontmatter machinery, so any keys the app
//! does not know about survive untouched.

use super::helpers::write_atomic;
use super::{Cave, CaveError};

impl Cave {
    /// Raw contents of `.granit/agent/system.md`; `Ok(None)` when the file
    /// does not exist.
    pub fn read_system_prompt_raw(&self) -> Result<Option<String>, CaveError> {
        match std::fs::read_to_string(self.system_prompt_path()) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomically write `.granit/agent/system.md`, creating `.granit/agent/`
    /// as needed.
    pub fn write_system_prompt(&self, content: &str) -> Result<(), CaveError> {
        std::fs::create_dir_all(self.agent_dir())?;
        write_atomic(&self.system_prompt_path(), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::cave::Cave;

    #[test]
    fn test_system_prompt_roundtrip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(cave.read_system_prompt_raw().unwrap(), None);

        cave.write_system_prompt("custom prompt {{ today }}")
            .unwrap();
        assert_eq!(
            cave.read_system_prompt_raw().unwrap().as_deref(),
            Some("custom prompt {{ today }}")
        );
        assert!(dir.path().join(".granit/agent/system.md").exists());
    }
}
