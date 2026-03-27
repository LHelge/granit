---
id: uyq
title: Implement recursive cave indexing and filename uniqueness checks
status: open
priority: P1
created: 2026-03-27T12:42:05.126552Z
updated: 2026-03-27T12:44:25.802934Z
depends_on:
- 5d2
parent: ph5
---

## Summary

Update the backend cave model to discover notes recursively and enforce the global filename-uniqueness rule across the entire cave tree. This provides the storage and validation foundation for all nested-folder behavior.

## Acceptance Criteria

- [ ] Cave note discovery works recursively across nested subfolders.
- [ ] Duplicate filenames anywhere in the cave are rejected or surfaced according to the agreed design.
- [ ] Backend note metadata preserves the information needed to distinguish root and nested notes.
- [ ] Existing flat-cave behavior continues to work.
- [ ] Tests cover recursive discovery and duplicate-name handling.

## Implementation Notes

- Focus first on src-tauri/src/cave/mod.rs and related cave data structures.
- Keep the filename-uniqueness rule centralized so create, rename, and move operations can reuse it.
- Coordinate with the title semantics decision so content-derived titles do not leak back into the model.

## Edge Cases

- Empty folders.
- Hidden folders or non-markdown files.
- Duplicate filenames found during initial scan.
- Mixed root and nested notes.

## Testing

- Add backend tests for recursive note discovery and duplicate filename rejection.
- cargo fmt && cargo clippy && cargo test must pass.
