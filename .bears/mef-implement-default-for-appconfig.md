---
id: mef
title: Implement Default for AppConfig
status: open
priority: P1
created: 2026-04-04T21:40:04.590475131Z
updated: 2026-04-04T21:40:04.590475131Z
tags:
- config
- backend
depends_on:
- xq8
parent: drc
---

## Summary
Implement `Default` for `AppConfig` as single source of truth for all default values. Currently defaults are scattered across `ensure_global()` and `merge()` with inconsistent values (e.g. theme `"dark"` vs `"default"`).

## What to do
- Add `impl Default for AppConfig` with canonical defaults
- Use it in `ensure_global()` instead of manual struct construction
- Use `#[serde(default)]` on `AppConfig` fields so deserialization fills missing fields
- Remove hardcoded default strings from multiple locations

## Fixes
- Theme default inconsistency (`"dark"` vs `"default"`)
- `daily_note_folder` was also duplicated

## Files
- `src-tauri/src/config/mod.rs`

## Testing
- Unit test: `AppConfig::default()` produces expected values
- Config file with missing fields deserializes correctly with defaults