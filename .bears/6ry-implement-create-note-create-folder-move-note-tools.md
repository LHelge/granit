---
id: 6ry
title: Implement create_note, create_folder, move_note tools
status: done
priority: P1
created: 2026-03-31T21:57:34.066112928Z
updated: 2026-04-01T13:53:52.522319Z
tags:
- backend
depends_on:
- 3sh
parent: qk2
---

## Summary

Implement the `create_note`, `create_folder`, and `move_note` tools. These are write operations that modify the cave structure.

## Acceptance Criteria

- [ ] `CreateNoteTool` — takes `{ name: String, folder: Option<String>, content: Option<String> }`, calls `Cave::create_note()` and optionally writes initial content via `Cave::save_note()`
- [ ] `CreateFolderTool` — takes `{ path: String }`, calls `Cave::create_folder()`
- [ ] `MoveNoteTool` — takes `{ slug: String, destination: Option<String> }`, calls `Cave::move_note()`
- [ ] All tools hold `Arc<Mutex<Option<Cave>>>` for cave access
- [ ] All require mutable access to `Cave` (`create_note`, `create_folder`, `move_note` take `&mut self`)
- [ ] Descriptive tool descriptions for LLM usage
- [ ] Unit tests
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/agent/tools/create_note.rs`, `src-tauri/src/agent/tools/create_folder.rs`, `src-tauri/src/agent/tools/move_note.rs`
- `create_note` and `move_note` take `&mut self` on Cave — need write lock
- `create_note` with content: call `create_note()` to get the slug, then `save_note()` to set content
- `move_note` destination `None` means move to cave root
- Consider emitting events to refresh the frontend's note/folder list after mutations