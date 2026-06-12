---
id: kxx
title: Fix partial update atomicity in cave operations
status: done
priority: P1
created: 2026-03-31T16:32:23.223567411Z
updated: 2026-03-31T16:47:09.445472382Z
tags:
- backend
- bug
parent: 4cm
---

## Summary
In `move_folder` and `update_note` (cave/mod.rs), the in-memory index is updated before the filesystem operation. If `fs::rename` or `fs::write` fails, the index is corrupted.

## Acceptance Criteria
- [ ] `move_folder`: `fs::rename` executes before index update; rollback index on failure
- [ ] `update_note`: if rename succeeds but write fails, rollback the rename
- [ ] Add tests simulating fs failure (e.g. read-only dir) to verify no index corruption

## Implementation Notes
- Files: `src-tauri/src/cave/mod.rs`
- Pattern: do filesystem op → on success update index → on failure return error with index unchanged
- Consider extracting a helper: `fn atomic_rename(&mut self, old: &Path, new: &Path) -> Result<()>`

## Testing
- `cargo test -p granit` must pass
- Add test: create note, make target dir read-only, attempt move → verify index unchanged