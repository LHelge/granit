//! Packing a cave directory into a tar + zstd archive.

use std::io::Write;
use std::path::Path;

use super::BackupError;

/// zstd level 3: good ratio on markdown at interactive speed.
const ZSTD_LEVEL: i32 = 3;

/// Paths (relative to the cave root) that never go into an archive:
/// the key cache must not be protected by itself, and the embedding
/// cache is regenerated from the notes anyway.
const EXCLUDED_FILES: [&str; 2] = [".granit/backup.key", ".granit/embeddings.bin"];

/// Directory names excluded wherever they appear. `.git` is regenerable
/// from its remote and can dwarf the notes themselves. Flip this to
/// include version history in backups.
const EXCLUDED_DIRS: [&str; 1] = [".git"];

fn is_excluded(relative: &Path) -> bool {
    let as_slash_path = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if EXCLUDED_FILES.contains(&as_slash_path.as_str()) {
        return true;
    }
    relative
        .components()
        .any(|c| EXCLUDED_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
}

/// Pack the whole cave (notes, attachments, and `.granit/` configuration,
/// minus the exclusions above) into an in-memory tar.zst archive.
///
/// In-memory single-shot is a deliberate simplification: a cave is a
/// personal markdown directory, far below 1 GB compressed. The container
/// format's version byte leaves room for a streamed v2 if that assumption
/// ever breaks.
pub(crate) fn pack_cave(cave_root: &Path) -> Result<Vec<u8>, BackupError> {
    let encoder = zstd::Encoder::new(Vec::new(), ZSTD_LEVEL)?;
    let mut builder = tar::Builder::new(encoder);
    append_dir(&mut builder, cave_root, cave_root)?;
    let encoder = builder.into_inner()?;
    Ok(encoder.finish()?)
}

/// Unpack a tar.zst archive (the decrypted payload of a backup container)
/// into `target`, which must already exist. `tar::Archive::unpack` refuses
/// entries that would escape the target directory.
pub(crate) fn unpack_cave(archive: &[u8], target: &Path) -> Result<(), BackupError> {
    let decoder = zstd::Decoder::new(archive)?;
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(target)?;
    Ok(())
}

fn append_dir<W: Write>(
    builder: &mut tar::Builder<W>,
    root: &Path,
    dir: &Path,
) -> Result<(), BackupError> {
    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    // Deterministic ordering makes archives (and tests) reproducible.
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("walk stays under the cave root");
        if is_excluded(relative) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // Explicit dir entries so empty folders survive a restore.
            builder.append_dir(relative, &path)?;
            append_dir(builder, root, &path)?;
        } else if file_type.is_file() {
            builder.append_path_with_name(&path, relative)?;
        }
        // Symlinks are skipped: a cave is expected to be plain files, and
        // following links could leak content from outside the cave.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::*;

    fn build_fixture_cave() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".granit/templates")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::create_dir_all(root.join("empty-folder")).unwrap();
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::write(root.join("note.md"), "# A note\n").unwrap();
        std::fs::write(root.join("projects/plan.md"), "# Plan\n").unwrap();
        std::fs::write(root.join("projects/image.png"), b"\x89PNG fake").unwrap();
        std::fs::write(root.join(".granit/config.yml"), "theme: default\n").unwrap();
        std::fs::write(root.join(".granit/templates/daily.md"), "# {{date}}\n").unwrap();
        std::fs::write(root.join(".granit/backup.key"), "SECRET").unwrap();
        std::fs::write(root.join(".granit/embeddings.bin"), b"\0\0cache").unwrap();
        std::fs::write(root.join(".git/objects/abc"), b"git blob").unwrap();
        dir
    }

    fn archive_paths(archive: &[u8]) -> BTreeSet<PathBuf> {
        let decoder = zstd::Decoder::new(archive).unwrap();
        let mut tar = tar::Archive::new(decoder);
        tar.entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect()
    }

    #[test]
    fn packs_notes_config_and_empty_dirs_but_not_secrets() {
        let cave = build_fixture_cave();
        let archive = pack_cave(cave.path()).unwrap();
        let paths = archive_paths(&archive);

        for expected in [
            "note.md",
            "projects/plan.md",
            "projects/image.png",
            ".granit/config.yml",
            ".granit/templates/daily.md",
            "empty-folder",
        ] {
            assert!(
                paths.contains(Path::new(expected)),
                "missing {expected} in {paths:?}"
            );
        }
        for excluded in [".granit/backup.key", ".granit/embeddings.bin"] {
            assert!(!paths.contains(Path::new(excluded)), "{excluded} leaked");
        }
        assert!(
            !paths.iter().any(|p| p.starts_with(".git")),
            ".git leaked into the archive"
        );
    }

    #[test]
    fn archived_contents_round_trip() {
        let cave = build_fixture_cave();
        let archive = pack_cave(cave.path()).unwrap();

        let restore = tempfile::tempdir().unwrap();
        let decoder = zstd::Decoder::new(&archive[..]).unwrap();
        tar::Archive::new(decoder).unpack(restore.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(restore.path().join("projects/plan.md")).unwrap(),
            "# Plan\n"
        );
        assert_eq!(
            std::fs::read_to_string(restore.path().join(".granit/config.yml")).unwrap(),
            "theme: default\n"
        );
        assert!(restore.path().join("empty-folder").is_dir());
        assert!(!restore.path().join(".granit/backup.key").exists());
    }

    #[test]
    fn packing_is_deterministic() {
        let cave = build_fixture_cave();
        let first = pack_cave(cave.path()).unwrap();
        let second = pack_cave(cave.path()).unwrap();
        // tar headers embed mtimes, which are stable here; ordering is what
        // this guards.
        assert_eq!(first, second);
    }
}
