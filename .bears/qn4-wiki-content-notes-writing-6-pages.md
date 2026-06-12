---
id: qn4
title: "Wiki content: Notes & Writing (6 pages)"
status: done
priority: P1
created: "2026-06-12T11:37:56.818090944Z"
updated: "2026-06-12T12:04:26.450739702Z"
tags:
  - docs
  - content
depends_on:
  - jfv
parent: sm3
---

## Summary

The six `category: Notes & Writing` wiki pages covering day-to-day note work.

## Acceptance Criteria

- [ ] `wiki/notes-and-markdown.md` — reader vs editor (CodeMirror), frontmatter fields (tags, timestamps, icon, favorite), sanitized HTML, mermaid support, interactive task checkboxes in reader.
- [ ] `wiki/wiki-links.md` — `[[note]]`, `[[note|label]]`, heading anchors via `{#id}` pandoc attributes, global anchor namespace (duplicate anchors refuse to open the cave), broken-link styling, backlinks.
- [ ] `wiki/templates.md` — `.granit/templates/`, creating notes from templates, flat template slug namespace.
- [ ] `wiki/daily-notes.md` — daily-note folder config, calendar strip, template seeding.
- [ ] `wiki/todos.md` — task checkboxes, todo sidebar tab, toggling from the reader.
- [ ] `wiki/explorer.md` — sidebar tabs: tree (drag-drop, rename, context menus), full-text search, tags, favorites.
- [ ] Cross-linked liberally between the six and to [[cave-rules]], [[configuration]].

## Implementation Notes

- Source: README.md feature bullets; CLAUDE.md "Cave model" + "Markdown" sections.
- Note for wiki-links.md: granit's bare global-anchor `[[Volvo]]` form is a granit feature being documented, not aphid link syntax — when *linking within the docs*, use aphid's `[[slug#anchor]]` form.