---
id: d2a
title: Add path-aware cave operations and folder management
status: open
priority: P1
created: 2026-03-27T12:42:05.213431Z
updated: 2026-03-27T12:44:25.918920Z
depends_on:
- uyq
parent: ph5
---

## Summary

Extend cave operations so notes and folders can be created, moved, renamed, and managed within a nested cave tree while preserving the global filename-uniqueness rule. This task turns the recursive model into a usable backend API.

## Acceptance Criteria

- [ ] Backend operations support nested note paths according to the agreed identity model.
- [ ] Folder creation and related path-aware operations are supported where required by the design.
- [ ] Rename and move behavior enforce global filename uniqueness consistently.
- [ ] Existing note CRUD behavior remains correct for root-level notes.
- [ ] Tests cover path-aware note and folder workflows.

## Implementation Notes

- Review command-facing cave methods in src-tauri/src/cave/mod.rs and how they are exposed from src-tauri/src/lib.rs.
- Keep handler code thin and move path logic into cave-layer helpers.
- Reuse recursive indexing and duplicate detection instead of reimplementing validation in each operation.

## Edge Cases

- Moving a note into a new folder.
- Renaming a note while keeping it in place.
- Renaming or moving into a conflicting filename anywhere in the cave.
- Operations on missing or partially created folders.

## Testing

- Add backend tests for path-aware create, move, rename, and conflict handling.
- cargo fmt && cargo clippy && cargo test must pass.
