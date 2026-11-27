---
id: zem.ub
title: "Sidebar polish: toggle, resize, note count, empty-space menu"
status: open
priority: P3
created: "2026-11-27T02:06:51.052472492Z"
updated: "2026-11-27T02:06:51.052472492Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4k
---

## Summary

Smaller sidebar features of the current app missing in the GPUI app (zem.5c inventory).

## Acceptance Criteria

- [ ] Sidebar show/hide toggle, saved in `sidebar.visible`
- [ ] Sidebar resize handle (320–600 px), saved in `sidebar.width`
- [ ] Note count footer ("N notes")
- [ ] Empty-space context menu on the tree: New note, New folder, New note from template

## Testing

- `gpui::test`s