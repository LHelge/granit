---
id: 5js
title: Add FontConfig types to config system
status: open
priority: P1
created: 2026-03-27T21:37:56.127460951Z
updated: 2026-03-29T21:02:47.425271058Z
tags:
- backend
- types
depends_on:
- hvc
parent: bp2
---

## Summary

Add `FontConfig` struct with `font_family` and `font_size` fields. Add three instances to `AppConfig`: `markdown_font`, `reading_font`, `agent_font`. Wire through the backend config (YAML load/save/merge).

## Acceptance Criteria

- [ ] `FontConfig { font_family: String, font_size: u8 }` in `granit-types`
- [ ] `AppConfig` gains `markdown_font`, `reading_font`, `agent_font` fields
- [ ] Backend `RawConfig` and merge logic updated
- [ ] Defaults: markdown=monospace/14, reading=sans-serif/16, agent=sans-serif/14
- [ ] `save_config` command accepts font configs
- [ ] Existing configs without font keys load gracefully

## Testing

- Unit tests for config load/merge with missing font sections
- `cargo fmt && cargo clippy && cargo test`