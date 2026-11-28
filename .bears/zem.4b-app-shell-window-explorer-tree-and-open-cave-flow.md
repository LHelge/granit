---
id: zem.4b
title: "App shell: window, explorer tree and open-cave flow"
status: done
priority: P1
created: "2026-11-26T21:09:11.701295Z"
updated: "2026-11-27T00:26:21.628104247Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4q
attempts: 1
---

## Summary

Start the real GPUI app crate: the main window, Catppuccin theming hook, sidebar explorer tree of notes and folders, and opening a cave (picker, last-open cave remembered, seeding `.granit/` as the current open-cave flow does). This is the skeleton every later frontend task builds on.

## Acceptance Criteria

- [ ] A new app crate (clean, not the spike crate) opens a window with a sidebar and a main area
- [ ] Opening a cave: folder picker on first run; the last-open cave path is persisted and reopened on start, without `tauri-plugin-store`
- [ ] The explorer shows nested folders and notes, with per-note icons and favorites as today, and selecting a note opens it in the main area (a placeholder view until the editor task lands)
- [ ] Create, rename, move and delete notes and folders from the explorer, through the core crate
- [ ] Cave mutations made elsewhere (agent tools, later) refresh the tree; the active note is known to the core state so agent tools can see it
- [ ] The app starts with a cave path argument for testing, so it never has to touch the live cave

## Implementation Notes

- Reuse from the spike selectively: `granit-gpui/src/sidebar.rs`, `tree_model.rs`, `theme.rs` and `themes.json` on `spike/gpui`.
- Current behaviour to match lives in `src/app/` (explorer components) and `src-tauri/src/commands/cave.rs`.
- Pick a file-dialog crate (e.g. `rfd`) and a small config-dir store for the last-open path.
- The app crate's binary name and bundle id matter for the updater and signing tasks; agree them here.

## Testing

- `gpui::test`s for opening a cave, tree contents and selection, as in the spike's `main.rs` tests