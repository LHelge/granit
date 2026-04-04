---
id: eup
title: Fix wrong error types in AppState helpers
status: open
priority: P1
created: 2026-04-04T21:40:23.639903961Z
updated: 2026-04-04T21:40:23.639903961Z
tags:
- errors
- backend
depends_on:
- 6wq
parent: drc
---

## Summary
`set_cave()`, `reset_agent()`, and `active_cave_path()` in `AppState` return `ConfigError` for non-config lock failures. This is semantically wrong.

## What to do
- Option A: Introduce a shared `StateError` enum used by all AppState lock helpers
- Option B: Each method returns its domain error (CaveError, AgentError, etc.)
- Option A is cleaner since these are all "mutex poisoned" errors from the same struct

## Also fix
- `select_provider()` creates fake `std::io::Error(InvalidInput)` for out-of-range index → add `ConfigError::Validation(String)` variant
- `open_daily_note()` maps `lock_config()` error to `CaveError::Io("Failed to lock config")` → use proper error type

## Files
- `src-tauri/src/lib.rs` — AppState methods, select_provider, open_daily_note
- `src-tauri/src/config/error.rs` — add Validation variant or create StateError