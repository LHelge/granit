---
id: 83x
title: Remove custom Theme/ThemeRegistry backend and shared types
status: done
priority: P1
created: 2026-04-04T20:03:33.476372952Z
updated: 2026-04-04T20:50:11.933118225Z
tags:
- backend
- types
depends_on:
- qmx
parent: uqp
---

## Summary

Remove the entire custom theme infrastructure from both backend and shared types. DaisyUI handles themes via `data-theme` — the backend only needs to store the theme name string, not a full color palette.

## Implementation Notes

### Backend removals:
1. **Delete `src-tauri/src/theme/mod.rs`** — `ThemeRegistry` is no longer needed
2. **Remove `theme` module** from `src-tauri/src/lib.rs` (`mod theme;` and `use theme::ThemeRegistry;`)
3. **Remove `theme_registry` field** from `AppState` struct
4. **Remove commands** `list_themes` and `get_active_theme` — no longer needed
5. **Simplify `set_active_theme`** — just validate the theme name is in the known list of 35 daisyUI themes, save to config. Could use a simple `const` array of valid theme names instead of a registry.
6. **Update `run()` builder** — remove `ThemeRegistry::new()` from `AppState`, remove `list_themes` and `get_active_theme` from `generate_handler!`

### Shared types removals (`granit-types/src/theme.rs`):
1. **Remove `Theme` struct** (full palette with 19 color fields)
2. **Remove `ThemeMeta` struct** — replace with a simpler approach: frontend knows the 35 daisyUI theme names directly
3. **Remove `builtin_themes()`, `theme_default()`, `theme_latte()`, etc.** — all 5 theme functions
4. **Update `granit-types/src/lib.rs`** — remove `mod theme` and its re-exports

### Frontend IPC removals (`src/app/ipc.rs`):
1. **Remove `list_themes()`** IPC wrapper
2. **Remove `get_active_theme()`** IPC wrapper
3. **Remove `Theme` and `ThemeMeta` imports** from granit-types

### Config changes:
- `AppConfig.theme` field stays as `String` — now stores a daisyUI theme name (e.g. "dark", "nord") instead of custom id (e.g. "mocha")
- Default value changes from `"default"` to `"dark"` in both backend `AppConfig` and IPC `AppConfig`

## Files to Modify
- `src-tauri/src/theme/mod.rs` — DELETE entirely
- `src-tauri/src/lib.rs` — remove theme module, ThemeRegistry, list_themes, get_active_theme commands
- `granit-types/src/theme.rs` — DELETE entirely
- `granit-types/src/lib.rs` — remove theme module re-exports
- `src/app/ipc.rs` — remove list_themes, get_active_theme wrappers
- `src-tauri/src/config/mod.rs` — change default theme from "default" to "dark"
- `granit-types/src/config.rs` — change default theme from "default" to "dark"

## Acceptance Criteria
- [ ] No `Theme`, `ThemeMeta`, or `ThemeRegistry` types remain
- [ ] `list_themes` and `get_active_theme` commands removed
- [ ] `set_active_theme` validates against known daisyUI theme names
- [ ] Default theme is "dark"
- [ ] `cargo test -p granit` and `cargo test -p granit-types` pass
- [ ] `cargo clippy` clean