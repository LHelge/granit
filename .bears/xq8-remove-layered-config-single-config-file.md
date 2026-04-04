---
id: xq8
title: Remove layered config, single config file
status: in_progress
priority: P1
created: 2026-04-04T21:39:49.106444316Z
updated: 2026-04-04T22:08:38.764424723Z
tags:
- config
- backend
parent: drc
---

## Summary
Remove the layered config system (global + cave override). A single `~/.config/granit/config.yml` is the only config file. Cave `.granit/` directory keeps only `.gitignore`.

## What to remove
- `RawConfig` struct and all `Option<T>` wrapper fields
- `MergeRaw` trait and its 3 implementations
- `merge()`, `apply_raw_overrides()`, `load_raw()` functions
- `RawAgentConfig`, `RawFontConfig`, `RawSidebarConfig` structs
- Cave-level `config.yml` creation in `ensure_cave()`
- The `cave_path: Option<&Path>` parameter from `load()`

## What to simplify
- `load()` becomes: read file → deserialize into `AppConfig` (with serde defaults)
- `save_global()` becomes: serialize `AppConfig` directly (no Some-wrapping into RawConfig)
- `ensure_cave()` becomes: just create `.granit/` dir + `.gitignore`
- `open_cave()` in lib.rs: no config reload/merge on cave open

## Files
- `src-tauri/src/config/mod.rs` — main changes
- `src-tauri/src/lib.rs` — simplify `open_cave()` command

## Testing
- `cargo test -p granit` must pass
- Verify existing config files still load (serde defaults handle missing fields)