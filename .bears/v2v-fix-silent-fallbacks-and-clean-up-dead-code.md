---
id: v2v
title: Fix silent fallbacks and clean up dead code
status: open
priority: P2
created: 2026-04-04T21:41:28.867128410Z
updated: 2026-04-04T21:41:28.867128410Z
tags:
- idiomatic
- backend
parent: drc
---

## Summary
`render_markdown()` in lib.rs silently swallows a lock failure with `.ok()` and renders without wiki-links. User gets broken output with no indication why.

## What to do
- At minimum: log the lock error with `eprintln!` or `tracing::warn!`
- Optionally: propagate the error (change return type to Result)

## Also address
- `ConfigError::Poisoned` is defined in config/error.rs but only used in lib.rs AppState — move to a shared location or remove from config module
- `active_slug` field in cave: verify it's used by the frontend via `set_active_note` IPC; if so, document

## Files
- `src-tauri/src/lib.rs` — render_markdown()
- `src-tauri/src/config/error.rs` — Poisoned variant
- `src-tauri/src/cave/mod.rs` — active_slug documentation