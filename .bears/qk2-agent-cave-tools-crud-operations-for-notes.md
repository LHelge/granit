---
id: qk2
title: Agent cave tools — CRUD operations for notes
type: epic
status: done
priority: P1
created: 2026-03-31T21:56:40.231849154Z
updated: 2026-04-01T14:09:22.812427Z
---

## Scope

Add cave CRUD tools to the rig-core agent so the AI can read, edit, create, and move notes within the currently open cave. Each tool implements `rig::tool::Tool` and operates on the shared `Cave` state.

## Tools to implement

1. **`read_active_note`** — Read the content of the note currently open in the editor
2. **`edit_active_note`** — Replace text in the active note (find & replace)
3. **`read_note`** — Read a note by slug
4. **`edit_note`** — Replace text in a note by slug (find & replace)
5. **`create_note`** — Create a new note (with optional folder)
6. **`create_folder`** — Create a folder in the cave
7. **`move_note`** — Move a note to a different folder

## Key Design Decisions

- Tools get `Arc<Mutex<Cave>>` (shared with `AppState`) for cave access
- Tools that need the "active note" also need `Arc<Mutex<Option<String>>>` (the currently selected slug, tracked by the frontend)
- `edit_note` / `edit_active_note` use the existing `Cave::edit_note(slug, old_text, new_text)` method — find & replace, not full overwrite
- Tools are registered on the agent during build via `.tool(ReadNoteTool { cave: arc_cave.clone() })`
- Each tool is a separate struct in `src-tauri/src/agent/tools/`
- Tool errors are mapped to strings for rig compatibility

## Acceptance Criteria

- [ ] All 7 tools implement `rig::tool::Tool` trait
- [ ] Tools are registered on the agent at build time
- [ ] Agent can read, create, edit, and move notes via natural language
- [ ] Active note tools work with the currently selected note in the editor
- [ ] Frontend emits active note changes so backend tracks them
- [ ] Error cases handled gracefully (no cave open, note not found, etc.)
- [ ] `cargo fmt && cargo clippy && cargo test` pass