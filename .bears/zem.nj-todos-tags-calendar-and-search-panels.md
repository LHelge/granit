---
id: zem.nj
title: Todos, tags, calendar and search panels
status: open
priority: P2
created: "2026-11-26T21:09:59.285080Z"
updated: "2026-11-26T21:09:59.285080Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4b
---

## Summary

Port the side panels that browse the cave: open todos across notes, tags, the calendar of daily notes, and full-text search. Each panel lists results from the core crate and opens the note (at the right spot where the current app does).

## Acceptance Criteria

- [ ] Todos panel: lists open tasks across the cave; toggling one edits the source note through the core crate
- [ ] Tags panel: lists tags with counts; selecting one lists its notes
- [ ] Calendar: month view of daily notes; clicking a day opens or creates that day's note
- [ ] Search: results update as the user types and open the note at the match
- [ ] Panels refresh after cave mutations

## Implementation Notes

- Current code: the matching components in `src/app/` and commands in `src-tauri/src/commands/`; the queries already live in the backend, so this task should be frontend-only.
- GPUI Kit's list and tree views cover the list UIs; a calendar may need a small custom element.
- Split this task per panel if it grows past one session.

## Testing

- `gpui::test`s per panel against a fixture cave