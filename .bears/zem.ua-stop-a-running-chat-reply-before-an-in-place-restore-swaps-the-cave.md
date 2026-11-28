---
id: zem.ua
title: Stop a running chat reply before an in-place restore swaps the cave
status: open
priority: P2
created: "2026-11-28T08:47:42.830168797Z"
updated: "2026-11-28T08:47:42.830168797Z"
tags:
  - gpui
  - port
  - agent
  - backup
---

## Summary

Found while implementing `zem.q5`. A cave switch now stops a running chat reply before the core swaps caves (`ChatPanel::stop_reply`, called from the workspace's `before_install` hook). An in-place backup restore (`Workspace::restore_backup`) closes and swaps the cave without stopping the reply, so the reply's agent tools can act on no cave, or on the restored one.

## Acceptance Criteria

- [ ] An in-place restore stops a running reply before the cave is closed and its folder swapped
- [ ] A restore that is refused (pending edits cannot be written) leaves the reply running

## Implementation Notes

- Call `ChatPanel::stop_reply` in `Workspace::restore_backup` after `save_before_switch` succeeds, where the note view is locked.
- Not verified in `zem.q5`: tokio's abort takes effect at the next await point, so a tool step already being polled can still finish.

## Testing

- A `#[gpui::test]` like `a_cave_switch_stops_the_reply_before_the_core_swaps_caves`, for the in-place restore