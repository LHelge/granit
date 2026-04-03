---
id: 6uq
title: Carry icons through cave and IPC flows
status: done
priority: P1
created: 2026-04-03T14:10:19.975947105Z
updated: 2026-04-03T20:17:01.927030798Z
tags:
- backend
- ipc
- notes
depends_on:
- 79f
parent: gcy
---

## Summary

Propagate note icon metadata through cave operations and Tauri IPC so note lists, reads, saves, renames, moves, and updates all preserve and return the optional icon needed by the tree view and editor.

## Acceptance Criteria
- [ ] `list_notes()` returns `NoteMeta.icon` for each note.
- [ ] `read_note`, `create_note`, `save_note`, `rename_note`, `move_note`, and `update_note` all preserve or return icon metadata correctly.
- [ ] Frontmatter rebuild/update logic can override `icon` alongside tags while updating `modified_at`.
- [ ] Frontend IPC wrappers support the extended update payload.

## Implementation Notes
- Update cave metadata helpers in `src-tauri/src/cave/`.
- Extend `rebuild_with_frontmatter` and note update/save flows in `src-tauri/src/markdown/mod.rs` and `src-tauri/src/cave/mod.rs`.
- Update Tauri command boundaries in `src-tauri/src/lib.rs` and wrappers in `src/app/ipc.rs`.
- Accept the extra frontmatter reads in `list_notes()` for now.

## Testing
- Add coverage for icon persistence through update/list flows.
- `cargo fmt && cargo clippy && cargo test` should pass.
