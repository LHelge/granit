---
id: zem.2u
title: Settings view with Catppuccin light and dark
status: open
priority: P2
created: "2026-11-26T21:09:38.877282Z"
updated: "2026-11-26T21:09:38.877282Z"
tags:
  - gpui
  - port
  - frontend
  - settings
depends_on:
  - zem.4b
---

## Summary

A settings view for the cave-local config in `.granit/config.yml`: theme (Catppuccin light and dark, following the system by default), editor and chat fonts, daily-note settings, sidebar, and agent settings (providers with inline API keys, default model, disabled tools, RAG options, system prompt).

## Acceptance Criteria

- [ ] Every setting the current settings pane exposes is editable, except theme choices beyond Catppuccin light/dark
- [ ] Changes are written to the cave's `.granit/config.yml` through the core crate and applied live (theme, fonts, agent rebuild, RAG reindex as today)
- [ ] `active_cave` is never written to cave YAML
- [ ] API key fields are masked

## Implementation Notes

- Current pane: `src/app/` settings components and `src-tauri/src/commands/config.rs`.
- Existing caves may have a DaisyUI theme name in config; map unknown names to the nearest Catppuccin flavour rather than failing.

## Testing

- Config round-trip tests in the core crate; a `gpui::test` that a theme change re-themes the editor