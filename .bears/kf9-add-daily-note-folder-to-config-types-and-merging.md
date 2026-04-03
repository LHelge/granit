---
id: kf9
title: Add daily_note_folder to config types and merging
status: done
priority: P1
created: 2026-04-03T23:08:35.230024144Z
updated: 2026-04-03T23:16:13.374513857Z
tags:
- config
- backend
- types
parent: hh6
---

## Summary
Add `daily_note_folder: String` to the shared config types and backend config with layered merging support.

## Acceptance Criteria
- [ ] `granit-types/src/config.rs` — `AppConfig` gains `daily_note_folder: String` field, default `"Daily"`
- [ ] `src-tauri/src/config/mod.rs` — `RawConfig` gains `daily_note_folder: Option<String>`, merged in `MergeRaw`
- [ ] Backend `AppConfig` gains matching field, default, and merge logic
- [ ] Existing tests pass, new unit test for merge precedence

## Implementation Notes
- `granit-types/src/config.rs`: add field to `AppConfig` struct and its `Default` impl
- `src-tauri/src/config/mod.rs`: add `daily_note_folder: Option<String>` to `RawConfig`, handle in `MergeRaw` impl
- Follow the exact pattern used by `sidebar` / `agent_panel` fields for layered merging
- Test: global sets "Journal", cave overrides to "Diary" → merged result is "Diary"

## Testing
- Unit test in `src-tauri/src/config/mod.rs` tests module