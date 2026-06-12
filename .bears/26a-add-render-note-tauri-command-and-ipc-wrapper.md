---
id: 26a
title: Add render_note Tauri command and IPC wrapper
status: done
priority: P1
created: 2026-03-27T21:25:43.985489914Z
updated: 2026-03-27T22:36:29.805590581Z
tags:
- backend
- frontend
depends_on:
- qvu
parent: z69
---

## Summary

Add a `render_note` Tauri command that reads a note, runs it through the full markdown pipeline, and returns rendered HTML + metadata to the frontend.

## Acceptance Criteria

- [ ] New `#[tauri::command] fn render_note(slug: &str, state) -> Result<RenderedNote, _>` command
- [ ] Reads note content from the cave via existing `cave` module
- [ ] Passes content + cave note list through the markdown pipeline (frontmatter → wiki-links → pulldown-cmark)
- [ ] Returns `RenderedNote { html, frontmatter, outgoing_links }` 
- [ ] `RenderedNote` type added to `granit-types` so the frontend can deserialize it
- [ ] Command registered in the Tauri invoke handler
- [ ] Frontend IPC wrapper `ipc::render_note(slug)` added

## Implementation Notes

- Files: `src-tauri/src/lib.rs` (command + registration), `src/app/ipc.rs` (frontend wrapper), `granit-types/src/lib.rs` (RenderedNote type)
- Keep the command handler thin — delegate to `markdown::render()`
- The command needs access to `AppState` to get the cave path and note list

## Testing

- Integration-level: verify the command wires up correctly (manual or via `cargo test`)
- Existing unit tests in the markdown module cover rendering correctness