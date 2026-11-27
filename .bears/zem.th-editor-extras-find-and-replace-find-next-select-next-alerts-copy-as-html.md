---
id: zem.th
title: "Editor extras: find and replace, find next, select next, alerts, copy as HTML"
status: open
priority: P3
created: "2026-11-27T02:06:51.073856741Z"
updated: "2026-11-27T02:06:51.073856741Z"
tags:
  - gpui
  - port
  - frontend
  - editor
depends_on:
  - zem.4k
---

## Summary

Editor features of the current app missing in the GPUI app (zem.5c inventory).

## Acceptance Criteria

- [ ] A find button opens the find bar; find next/previous shortcuts (Ctrl/Cmd+G, Shift+Ctrl/Cmd+G)
- [ ] Find and replace
- [ ] Select next occurrence (zorite binds Ctrl+D to Duplicate; pick a binding matching `js/editor.ts`)
- [ ] `> [!` alert completion
- [ ] "Copy note" as rich HTML (`copy_note_html`), if GPUI's clipboard supports HTML; else record it

## Testing

- `gpui::test`s