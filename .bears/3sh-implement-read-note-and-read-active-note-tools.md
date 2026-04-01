---
id: 3sh
title: Implement read_note and read_active_note tools
status: done
priority: P1
created: 2026-03-31T21:57:13.877817327Z
updated: 2026-04-01T13:53:52.514486Z
tags:
- backend
depends_on:
- kgh
parent: qk2
---

## Summary

Create the `src-tauri/src/agent/tools/` module with the `rig::tool::Tool` infrastructure and implement the two read-only tools: `read_note` (by slug) and `read_active_note`.

## Acceptance Criteria

- [ ] `src-tauri/src/agent/tools/mod.rs` — module declaration, re-exports
- [ ] `ReadNoteTool` — takes `{ slug: String }`, returns note content + metadata as JSON string
- [ ] `ReadActiveNoteTool` — takes no args (or empty struct), reads the active note slug from state, then reads that note
- [ ] Both tools hold `Arc<Mutex<Option<Cave>>>` for cave access
- [ ] `ReadActiveNoteTool` also holds `Arc<Mutex<Option<String>>>` for active note
- [ ] Tools return descriptive errors when cave not open or note not found
- [ ] Good tool descriptions (the LLM reads these to decide when to call)
- [ ] Unit tests for tool logic (mock cave via tempdir)
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/agent/tools/mod.rs`, `src-tauri/src/agent/tools/read_note.rs`, `src-tauri/src/agent/tools/read_active_note.rs`
- rig `Tool` trait requires: `const NAME`, `type Args` (Deserialize), `type Output` (Serialize + Send + Sync), `type Error` (thiserror), `async fn call(&self, args) -> Result<Output, Error>`
- Also needs `definition()` method returning `ToolDefinition` with name, description, and JSON schema of args
- Tool output should be a string (rendered for the LLM context) — consider returning markdown content directly
- Error type can be a simple `#[derive(thiserror::Error)] ToolError(String)`