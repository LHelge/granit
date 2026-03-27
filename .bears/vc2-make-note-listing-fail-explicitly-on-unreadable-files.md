---
id: vc2
title: Make note listing fail explicitly on unreadable files
status: done
priority: P2
created: 2026-03-27T12:20:40.992694Z
updated: 2026-03-27T13:13:41.864210Z
depends_on:
- c2z
parent: wem
---

## Summary

Make `list_notes` report file-read problems explicitly instead of synthesizing note metadata from empty content. The current implementation hides unreadable-file failures and returns misleading titles and partial success.

## Acceptance Criteria

- [ ] `list_notes` no longer falls back to empty content for unreadable markdown files.
- [ ] The backend has a clear and consistent failure policy for unreadable files during note listing.
- [ ] The chosen behavior is reflected in tests.
- [ ] The frontend can distinguish a list failure from an empty cave once IPC error handling is normalized.

## Implementation Notes

- Review src-tauri/src/cave/mod.rs, especially the current `read_to_string(...).unwrap_or_default()` path.
- Prefer an explicit error over silent partial success unless a stronger product requirement emerges.
- Coordinate any frontend behavior changes with the IPC error handling task.

## Edge Cases

- File exists but is unreadable due to permissions.
- File is deleted between directory iteration and content read.
- A cave contains one unreadable file among many valid notes.

## Testing

- Add backend tests for unreadable or otherwise failing note-list reads where the platform allows it.
- cargo fmt && cargo clippy && cargo test must pass.
