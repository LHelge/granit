---
id: gte
title: Render custom note icons in tree view
status: open
priority: P1
created: 2026-04-03T14:10:32.700934109Z
updated: 2026-04-03T14:10:32.700934109Z
tags:
- frontend
- tree
- notes
depends_on:
- 6uq
- cdq
parent: gcy
---

## Summary

Replace the hardcoded note file icon in the tree view with the shared note icon helper so each note row renders its custom icon when present and falls back to `LuFileText` otherwise.

## Acceptance Criteria
- [ ] Tree note rows render `meta.icon` when present.
- [ ] Notes without an icon still render `LuFileText`.
- [ ] Unknown icon ids do not break rendering and use the fallback icon.

## Implementation Notes
- Update `src/app/components/tree_view/note_node.rs`.
- Reuse the shared catalog/render helper instead of duplicating icon mapping logic.
