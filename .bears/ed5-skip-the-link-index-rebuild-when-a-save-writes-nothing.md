---
id: ed5
title: Skip the link-index rebuild when a save writes nothing
status: open
priority: P3
created: "2026-11-27T00:35:36.693573865Z"
updated: "2026-11-27T00:35:36.693573865Z"
tags:
  - backend
  - core
---

## Summary

Since zem.ry, `Cave::save_note` skips the write when nothing changed, but it still runs `rebuild_link_indexes` (a whole-cave scan) afterwards. Skip the rebuild when no file was written.

## Acceptance Criteria

- [ ] An unedited save does no whole-cave scan; an edited save still updates backlinks and anchors

## Testing

- A test that an unedited save leaves the indexes untouched (or counts rebuilds)