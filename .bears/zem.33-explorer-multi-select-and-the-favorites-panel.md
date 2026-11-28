---
id: zem.33
title: Explorer multi-select and the favorites panel
status: done
priority: P2
created: "2026-11-27T00:35:36.663905246Z"
updated: "2026-11-27T05:41:18.373874997Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4b
attempts: 1
---

## Summary

The GPUI explorer (zem.4b) left out two parts of the current explorer: multi-select with ctrl/shift-click (and dragging several notes at once), and the separate Favorites panel (only the star in the tree exists). Port both so the explorer matches today.

## Acceptance Criteria

- [ ] Ctrl/Cmd-click toggles and Shift-click extends a selection in the tree; dragging a selection moves every selected note; delete acts on the selection as today
- [ ] A Favorites section lists favorite notes and opens them on click, refreshing on cave events

## Implementation Notes

- Current behaviour: the explorer and favorites components in `src/app/`.
- Explorer seams: `granit-app/src/explorer/`, `CaveEvents` in `granit-app/src/events.rs`.

## Testing

- `gpui::test`s for selection and the favorites list