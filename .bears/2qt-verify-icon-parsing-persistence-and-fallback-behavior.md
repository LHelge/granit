---
id: 2qt
title: Verify icon parsing, persistence, and fallback behavior
status: open
priority: P1
created: 2026-04-03T14:10:44.776058114Z
updated: 2026-04-03T14:10:44.776058114Z
tags:
- testing
- backend
- frontend
depends_on:
- gte
- 6rf
parent: gcy
---

## Summary

Add or update automated coverage for backend icon metadata handling and complete manual verification for the end-to-end icon workflow, including fallback and note lifecycle behaviors.

## Acceptance Criteria
- [ ] Backend tests cover icon frontmatter parsing and rebuild behavior.
- [ ] Backend tests cover list/update flows returning `NoteMeta.icon`.
- [ ] Manual verification covers selecting, clearing, reopening, renaming, and moving notes with icons.
- [ ] Unknown icon ids are verified to fall back safely to `LuFileText`.

## Implementation Notes
- Focus automated tests in `src-tauri/src/markdown/mod.rs`, `src-tauri/src/cave/mod.rs`, and related helpers.
- Use manual verification for the editor/grid UX unless frontend tests are already practical.
