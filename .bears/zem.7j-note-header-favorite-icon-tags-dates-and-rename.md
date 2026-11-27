---
id: zem.7j
title: "Note header: favorite, icon, tags, dates and rename"
status: open
priority: P2
created: "2026-11-27T02:06:51.011878071Z"
updated: "2026-11-27T02:06:51.011878071Z"
tags:
  - gpui
  - port
  - frontend
  - editor
depends_on:
  - zem.4k
---

## Summary

Port the current note header (found missing in the zem.5c inventory). The core already has `Cave::update_note`.

## Acceptance Criteria

- [ ] Favorite toggle in the header; the icon and star show next to the title
- [ ] Note icon picker: a searchable `NOTE_ICONS` grid
- [ ] Tags editor: tag pills and an "Add tag" input
- [ ] Created/modified dates, word count and reading time under the title
- [ ] Rename by editing the header title
- [ ] The same header for templates: a template's icon and tags can be edited (`Cave::update_template`), as the Tauri app allows
- [ ] Backlinks section under the title, as the current app shows it (the epic defers a backlinks *panel*; confirm with the owner whether this section counts)

## Testing

- `gpui::test`s per control; frontmatter edits change only their own key (zem.ry)