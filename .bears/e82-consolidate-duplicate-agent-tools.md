---
id: e82
title: Consolidate duplicate agent tools
status: in_progress
priority: P2
created: 2026-04-04T21:41:12.257098648Z
updated: 2026-04-04T22:41:32.558484009Z
tags:
- duplication
- backend
- agent
parent: drc
---

## Summary
`read_note` vs `read_active_note` and `edit_note` vs `edit_active_note` differ only in how the slug is resolved (explicit param vs active note). This is 2 files of pure duplication.

## What to do
- `read_note` tool: make `slug` optional. If None, use `cave.active_slug()`
- `edit_note` tool: make `slug` optional. If None, use `cave.active_slug()`
- Delete `read_active_note.rs` and `edit_active_note.rs`
- Update tool registration in `cave_toolset()`

## Files
- `src-tauri/src/agent/tools/read_note.rs`
- `src-tauri/src/agent/tools/edit_note.rs`
- `src-tauri/src/agent/tools/read_active_note.rs` (delete)
- `src-tauri/src/agent/tools/edit_active_note.rs` (delete)
- `src-tauri/src/agent/tools/mod.rs`