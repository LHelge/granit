---
id: hvc
title: Add EditorConfig to config system
status: open
priority: P1
created: 2026-03-27T21:37:46.400844004Z
updated: 2026-03-27T21:37:46.400844004Z
tags:
- backend
parent: bp2
---

## Summary

Add an `EditorConfig` struct to `granit-types` and wire it through the backend config system (global YAML, layered merge, save).

## Acceptance Criteria

- [ ] `EditorConfig { font_family: String, font_size: u8 }` added to `granit-types`
- [ ] `AppConfig` gains an `editor: EditorConfig` field
- [ ] Backend `RawConfig` and merge logic updated to handle the new section
- [ ] Defaults: `font_family: "monospace"`, `font_size: 14`
- [ ] `save_config` Tauri command updated to accept and persist editor settings
- [ ] Existing configs without the `editor` key load gracefully (defaults fill in)

## Implementation Notes

- Files: `granit-types/src/lib.rs`, `src-tauri/src/config/mod.rs`
- Follow the same pattern as `AgentConfig`: `Option` fields in `RawConfig`, merged with defaults
- `font_size` as `u8` is sufficient (range 8–32 is reasonable)

## Testing

- Unit test: config without `editor` section loads with defaults
- Unit test: config with partial `editor` section merges correctly
- `cargo fmt && cargo clippy && cargo test` must pass