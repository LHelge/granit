---
id: 8fz
title: Implement edit_note and edit_active_note tools
status: open
priority: P1
created: 2026-03-31T21:57:23.693483550Z
updated: 2026-03-31T21:57:23.693483550Z
tags:
- backend
depends_on:
- 3sh
parent: qk2
---

## Summary

Implement the two edit tools: `edit_note` (by slug) and `edit_active_note`. These perform find-and-replace operations on note content using the existing `Cave::edit_note()` method.

## Acceptance Criteria

- [ ] `EditNoteTool` — takes `{ slug: String, old_text: String, new_text: String }`, calls `Cave::edit_note()`
- [ ] `EditActiveNoteTool` — takes `{ old_text: String, new_text: String }`, resolves active note slug, then calls `Cave::edit_note()`
- [ ] Both return confirmation with the updated note metadata
- [ ] Both require `Arc<Mutex<Option<Cave>>>` (mutable access pattern via lock)
- [ ] `EditActiveNoteTool` also needs `Arc<Mutex<Option<String>>>` for active note slug
- [ ] Descriptive tool descriptions explaining find-and-replace semantics
- [ ] Unit tests
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/agent/tools/edit_note.rs`, `src-tauri/src/agent/tools/edit_active_note.rs`
- `Cave::edit_note` takes `&self` (not `&mut self`) so a read lock on the cave suffices
- Error on: cave not open, note not found, old_text not found in content
- After edit, frontend should be notified to refresh — consider emitting a Tauri event, or let the frontend poll. This can be a follow-up concern.