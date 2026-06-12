---
id: 6wq
title: Remove Cave variant from ConfigError
status: in_progress
priority: P1
created: 2026-04-04T21:40:12.865956467Z
updated: 2026-04-04T22:24:43.372743812Z
tags:
- errors
- backend
depends_on:
- xq8
parent: drc
---

## Summary
Remove `Cave(#[from] CaveError)` variant from `ConfigError`. Config module shouldn't depend on cave module — this is a layering violation. With the simplified config, this dependency is no longer needed.

## What to do
- Remove `Cave` variant from `ConfigError` enum in `config/error.rs`
- Remove `use crate::cave::CaveError` from config error module
- In `lib.rs`, handle cave errors at command level (map to appropriate type or use a shared error)

## Files
- `src-tauri/src/config/error.rs`
- `src-tauri/src/lib.rs` (adjust any command that relied on ConfigError::Cave)