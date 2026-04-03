---
id: t2e
title: Backend theme registry and Tauri commands
status: open
priority: P1
created: 2026-04-02T23:10:27.495520508Z
updated: 2026-04-02T23:10:27.495520508Z
tags:
- backend
depends_on:
- pba
parent: njk
---

## Summary
Add a backend theme registry module that loads compiled-in themes and exposes Tauri commands to list themes and get/set the active theme.

## Acceptance Criteria
- [ ] `src-tauri/src/theme/mod.rs` module with theme registry
- [ ] Tauri command `list_themes` → returns all available themes (id, name, dark flag)
- [ ] Tauri command `get_active_theme` → returns the full active theme (all colors)
- [ ] Tauri command `set_active_theme(id)` → switches theme, persists to config
- [ ] Active theme ID stored in global config and per-cave config (optional override)
- [ ] Default theme is Mocha if none configured

## Implementation Notes
- Registry initialized at app startup with the four embedded themes
- Manage as Tauri state (`State<ThemeRegistry>`)
- Wire commands in `lib.rs`
- Theme config field: `theme: Option<String>` in both global and cave config structs