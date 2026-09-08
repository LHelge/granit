//! Filesystem side of restore: unpack a decrypted archive into a fresh
//! directory, or swap it in place of the current cave.
//!
//! The in-place path never modifies the original cave directory: the
//! archive is unpacked into a temporary sibling first, and only two
//! renames swap it into place, so a failure at any earlier step (disk
//! full included) leaves the cave untouched.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{archive, crypto, BackupError};
use crate::backup::crypto::{CachedKey, KEY_FILE_NAME};

/// Check the downloaded container against the record's digest, then decrypt
/// it — silently with the cave's cached key when its salt and KDF parameters
/// match the archive header, with the passphrase otherwise. With neither, the
/// caller must prompt ([`BackupError::PassphraseRequired`]).
pub(crate) fn verify_and_decrypt(
    container: &[u8],
    expected_sha256_hex: &str,
    cached: Option<&CachedKey>,
    passphrase: Option<&str>,
) -> Result<Vec<u8>, BackupError> {
    if hex::encode(Sha256::digest(container)) != expected_sha256_hex {
        return Err(BackupError::ChecksumMismatch);
    }
    if let Some(cached) = cached {
        if crypto::key_matches(cached, container)? {
            return crypto::decrypt_with_cached(cached, container);
        }
    }
    match passphrase {
        Some(passphrase) => crypto::decrypt(passphrase, container),
        None => Err(BackupError::PassphraseRequired),
    }
}

/// Unpack into `target`, which must be a nonexistent path or an empty
/// directory. Creates the directory (and parents) as needed.
pub(crate) fn restore_to_new_dir(plaintext: &[u8], target: &Path) -> Result<(), BackupError> {
    if target.exists() {
        if !target.is_dir() {
            return Err(BackupError::TargetNotEmpty);
        }
        if std::fs::read_dir(target)?.next().is_some() {
            return Err(BackupError::TargetNotEmpty);
        }
    } else {
        std::fs::create_dir_all(target)?;
    }
    archive::unpack_cave(plaintext, target)
}

/// Replace the cave at `cave_root` with the archive contents.
///
/// Unpacks into a hidden temporary sibling, carries over
/// `.granit/backup.key` (excluded from archives by design), then renames
/// the old cave to `<name>.pre-restore-<timestamp>` and moves the new tree
/// into place. Returns the pre-restore path — the old contents are left
/// there as the user's escape hatch.
pub(crate) fn restore_in_place(plaintext: &[u8], cave_root: &Path) -> Result<PathBuf, BackupError> {
    let parent = cave_root
        .parent()
        .ok_or_else(|| BackupError::Io("cave has no parent directory".to_string()))?;
    let name = cave_root
        .file_name()
        .ok_or_else(|| BackupError::Io("cave has no directory name".to_string()))?
        .to_string_lossy()
        .into_owned();

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let temp = parent.join(format!(".{name}.restore-tmp-{stamp}"));
    let pre_restore = parent.join(format!("{name}.pre-restore-{stamp}"));
    if temp.exists() || pre_restore.exists() {
        return Err(BackupError::Io(format!(
            "restore staging path already exists: {}",
            pre_restore.display()
        )));
    }

    let staged = (|| -> Result<(), BackupError> {
        std::fs::create_dir(&temp)?;
        archive::unpack_cave(plaintext, &temp)?;
        let key_file = cave_root.join(".granit").join(KEY_FILE_NAME);
        if key_file.is_file() {
            let target_granit = temp.join(".granit");
            std::fs::create_dir_all(&target_granit)?;
            std::fs::copy(&key_file, target_granit.join(KEY_FILE_NAME))?;
        }
        Ok(())
    })();
    if let Err(err) = staged {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(err);
    }

    std::fs::rename(cave_root, &pre_restore)?;
    if let Err(err) = std::fs::rename(&temp, cave_root) {
        // Put the original cave back so a failed swap loses nothing.
        let _ = std::fs::rename(&pre_restore, cave_root);
        let _ = std::fs::remove_dir_all(&temp);
        return Err(err.into());
    }
    Ok(pre_restore)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::archive::pack_cave;

    fn packed_fixture() -> (tempfile::TempDir, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".granit")).unwrap();
        std::fs::create_dir_all(root.join("empty-folder")).unwrap();
        std::fs::write(root.join("note.md"), "# A note\n").unwrap();
        std::fs::write(root.join(".granit/config.yml"), "theme: default\n").unwrap();
        let archive = pack_cave(root).unwrap();
        (dir, archive)
    }

    #[test]
    fn verify_and_decrypt_checks_digest_then_picks_key_source() {
        let cached = crypto::fast_key("correct horse battery");
        let container = crypto::encrypt(&cached, b"cave contents").unwrap();
        let sha = hex::encode(Sha256::digest(&container));

        // Tampered bytes are rejected before any decryption is attempted.
        assert!(matches!(
            verify_and_decrypt(b"tampered", &sha, Some(&cached), None),
            Err(BackupError::ChecksumMismatch)
        ));

        // Matching cached key decrypts silently, no passphrase needed.
        assert_eq!(
            verify_and_decrypt(&container, &sha, Some(&cached), None).unwrap(),
            b"cave contents"
        );

        // Mismatched cached key (different salt) falls back to the
        // passphrase, and without one asks the caller to prompt.
        let mismatched = CachedKey {
            key: cached.key,
            salt: [9u8; 16],
            params: cached.params,
        };
        assert!(matches!(
            verify_and_decrypt(&container, &sha, Some(&mismatched), None),
            Err(BackupError::PassphraseRequired)
        ));
        assert_eq!(
            verify_and_decrypt(
                &container,
                &sha,
                Some(&mismatched),
                Some("correct horse battery")
            )
            .unwrap(),
            b"cave contents"
        );

        // No cached key at all (fresh machine): passphrase or prompt.
        assert!(matches!(
            verify_and_decrypt(&container, &sha, None, None),
            Err(BackupError::PassphraseRequired)
        ));
        assert!(matches!(
            verify_and_decrypt(&container, &sha, None, Some("wrong passphrase!")),
            Err(BackupError::Decrypt)
        ));
    }

    #[test]
    fn restores_into_nonexistent_and_empty_dirs() {
        let (_src, archive) = packed_fixture();

        let parent = tempfile::tempdir().unwrap();
        let fresh = parent.path().join("restored");
        restore_to_new_dir(&archive, &fresh).unwrap();
        assert_eq!(
            std::fs::read_to_string(fresh.join("note.md")).unwrap(),
            "# A note\n"
        );
        assert!(fresh.join("empty-folder").is_dir());

        let empty = tempfile::tempdir().unwrap();
        restore_to_new_dir(&archive, empty.path()).unwrap();
        assert!(empty.path().join("note.md").is_file());
    }

    #[test]
    fn refuses_nonempty_target() {
        let (_src, archive) = packed_fixture();
        let target = tempfile::tempdir().unwrap();
        std::fs::write(target.path().join("existing.txt"), "keep me").unwrap();
        assert!(matches!(
            restore_to_new_dir(&archive, target.path()),
            Err(BackupError::TargetNotEmpty)
        ));
        // The existing contents are untouched.
        assert_eq!(
            std::fs::read_to_string(target.path().join("existing.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn in_place_swap_preserves_key_and_old_contents() {
        let (_src, archive) = packed_fixture();

        let parent = tempfile::tempdir().unwrap();
        let cave = parent.path().join("my-cave");
        std::fs::create_dir_all(cave.join(".granit")).unwrap();
        std::fs::write(cave.join("old-note.md"), "# Old\n").unwrap();
        std::fs::write(cave.join(".granit/backup.key"), "KEYFILE").unwrap();

        let pre_restore = restore_in_place(&archive, &cave).unwrap();

        // New contents are in place, with the key carried over.
        assert_eq!(
            std::fs::read_to_string(cave.join("note.md")).unwrap(),
            "# A note\n"
        );
        assert!(!cave.join("old-note.md").exists());
        assert_eq!(
            std::fs::read_to_string(cave.join(".granit/backup.key")).unwrap(),
            "KEYFILE"
        );

        // The old cave survives untouched at the returned path.
        assert!(pre_restore.starts_with(parent.path()));
        assert_eq!(
            std::fs::read_to_string(pre_restore.join("old-note.md")).unwrap(),
            "# Old\n"
        );
        assert_eq!(
            std::fs::read_to_string(pre_restore.join(".granit/backup.key")).unwrap(),
            "KEYFILE"
        );
    }

    #[test]
    fn corrupt_archive_leaves_cave_untouched() {
        let parent = tempfile::tempdir().unwrap();
        let cave = parent.path().join("my-cave");
        std::fs::create_dir_all(&cave).unwrap();
        std::fs::write(cave.join("old-note.md"), "# Old\n").unwrap();

        assert!(restore_in_place(b"not a zstd stream", &cave).is_err());

        assert_eq!(
            std::fs::read_to_string(cave.join("old-note.md")).unwrap(),
            "# Old\n"
        );
        // No staging or pre-restore leftovers.
        let siblings: Vec<_> = std::fs::read_dir(parent.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(siblings, vec![std::ffi::OsString::from("my-cave")]);
    }
}
