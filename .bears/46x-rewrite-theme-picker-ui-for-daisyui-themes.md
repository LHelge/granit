---
id: 46x
title: Rewrite theme picker UI for daisyUI themes
status: done
priority: P1
created: 2026-04-04T20:04:26.236772470Z
updated: 2026-04-04T20:53:10.004696800Z
tags:
- frontend
depends_on:
- 83x
- axu
- zh9
parent: uqp
---

## Objective
Rewrite the settings theme picker to show all available DaisyUI themes and custom Catppuccin themes.

## Steps
1. Define a compile-time `const` array of all theme names in the frontend:
   - 35 built-in DaisyUI themes
   - 4 custom Catppuccin themes: `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `catppuccin-mocha`
   - Include metadata: name, display label, light/dark classification
2. Rewrite `theme.rs` settings component:
   - Grid of theme swatches (similar to DaisyUI's theme preview cards)
   - Clicking a swatch: instantly sets `data-theme` via `set_daisy_theme()` + saves via IPC
   - Current theme highlighted
3. Remove calls to `ipc::list_themes()` and `ipc::get_active_theme()` (deleted in `83x`)
4. Catppuccin themes should be visually grouped or labeled in the picker

## Depends on
- `83x` (backend cleanup removes old theme commands)
- `axu` (color migration complete — UI looks correct with daisyUI colors)
- `zh9` (Catppuccin themes must exist to preview)