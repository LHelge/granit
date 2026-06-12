---
id: fmh
title: Reorganize tool files by category
status: done
priority: P1
created: 2026-04-05T11:03:57.625215028Z
updated: 2026-04-05T11:08:25.815532517Z
tags:
- agent
- refactor
parent: fm9
---

## Summary
Reorganize existing tool files from one-file-per-tool into category-grouped files. No behavior change — pure refactor.

## File Mapping

**Delete:** `create_note.rs`, `delete_note.rs`, `edit_note.rs`, `list_notes.rs`, `read_note.rs`, `search_notes.rs`, `update_note.rs`, `web_fetch.rs`, `web_search.rs`

**Create:**
- `navigation.rs` — `ListNotesTool`, `SearchNotesTool` (moved from list_notes.rs, search_notes.rs)
- `reading.rs` — `ReadNoteTool` (moved from read_note.rs)
- `writing.rs` — `CreateNoteTool`, `UpdateNoteTool`, `EditNoteTool` (moved from create_note.rs, update_note.rs, edit_note.rs)
- `organization.rs` — `DeleteNoteTool` (moved from delete_note.rs)
- `web.rs` — `WebFetchTool`, `WebSearchTool` (moved from web_fetch.rs, web_search.rs)

**Update:** `mod.rs` — change `mod` declarations and `pub use` to match new files.

## Acceptance Criteria
- [ ] All existing tools compile and work identically
- [ ] mod.rs updated with new module declarations
- [ ] All existing tests still pass
- [ ] No behavior changes

## Testing
- cargo test -p granit (backend tests must pass unchanged)