---
id: kgh
title: Track active note slug in backend state
status: open
priority: P1
created: 2026-03-31T21:57:00.513351428Z
updated: 2026-03-31T21:57:00.513351428Z
tags:
- backend
- frontend
depends_on:
- dpx
parent: qk2
---

## Summary

Track the currently active (selected/open in editor) note slug in backend state so agent tools can reference "the active note" without the user specifying a slug.

## Acceptance Criteria

- [ ] `AppState` has `active_note: Arc<Mutex<Option<String>>>` field
- [ ] New Tauri command `set_active_note(slug: Option<String>)` updates the state
- [ ] Frontend calls `set_active_note` when the user opens/selects a note, and `set_active_note(None)` when no note is selected
- [ ] `active_note_arc()` method on `AppState` for tool construction
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/lib.rs` (state + command), frontend component that manages note selection (e.g., sidebar click handler, editor open)
- Keep it simple: just a slug string, not the full note content
- The frontend already tracks which note is open — just add an IPC call when it changes