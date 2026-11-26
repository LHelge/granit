---
id: csj
title: "fix(backup): keep the cave's .git when restoring in place"
status: open
priority: P1
created: "2026-11-26T23:42:04.971444645Z"
updated: "2026-11-26T23:42:04.971444645Z"
tags:
  - backend
  - bug
---

## Summary

Restoring a backup with "Replace current cave" loses the cave's `.git` directory. Archives exclude `.git` (`EXCLUDED_DIRS` in `src-tauri/src/backup/archive.rs`). The in-place restore (`backup/restore.rs`) moves the current cave to `<cave>.pre-restore-*` and copies only `.granit/backup.key` back into the restored tree. A git-versioned cave therefore comes back without its history, and the docs recommend keeping caves under version control (`docs/content/wiki/agent-tools-and-rag.md`).

## Acceptance Criteria

- [ ] An in-place restore carries `.git` over from the replaced cave into the restored one, alongside `backup.key`. It is moved, not copied, when both are on the same filesystem.
- [ ] A restore into a new folder is unchanged.
- [ ] The roll-back copy `<cave>.pre-restore-*` still holds everything else of the old cave.
- [ ] Tests in `backup/restore.rs`:
  - an in-place restore of a cave with a `.git` directory keeps it;
  - a cave without one still restores;
  - a failure while carrying `.git` over leaves the pre-restore copy intact.
- [ ] `docs/content/wiki/backups.md` says that `.git` is kept on an in-place restore.

## Edge Cases

- `.git` as a file (a worktree or submodule pointer), not a directory.
- A restore that fails after the move: the roll-back must still put the original cave, `.git` included, back in place.