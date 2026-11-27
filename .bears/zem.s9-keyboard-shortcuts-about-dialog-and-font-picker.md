---
id: zem.s9
title: Keyboard shortcuts, about dialog and font picker
status: open
priority: P3
created: "2026-11-27T02:06:51.096035959Z"
updated: "2026-11-27T02:06:51.096035959Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4k
---

## Summary

Dialogs of the current app missing in the GPUI app (zem.5c inventory).

## Acceptance Criteria

- [ ] Keyboard-shortcuts dialog listing the GPUI app's bindings
- [ ] About dialog: version, git hash and dirty badge (set `GRANIT_GIT_HASH` / `GRANIT_GIT_DIRTY` in granit-app's build)
- [ ] Font picker with search and a live preview in the settings font fields

## Testing

- `gpui::test`s