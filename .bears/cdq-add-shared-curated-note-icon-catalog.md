---
id: cdq
title: Add shared curated note icon catalog
status: open
priority: P2
created: 2026-04-03T14:10:28.391594707Z
updated: 2026-04-03T14:10:28.391594707Z
tags:
- frontend
- ui
- icons
depends_on:
- 79f
parent: gcy
---

## Summary

Create a shared frontend icon catalog and render helper for note icons so both the tree view and the editor picker use the same curated Lucide subset, normalized ids, labels, and search tags.

## Acceptance Criteria
- [ ] A curated Lucide icon set is defined in one shared module.
- [ ] Each entry includes a normalized id, label, and extra search tags.
- [ ] A single helper maps icon ids to the corresponding Lucide icon and falls back to `LuFileText`.
- [ ] Unknown ids render safely with the default icon.

## Implementation Notes
- Place the catalog in a reusable frontend module near existing icon helpers.
- Follow existing `leptos_icons` usage rules: wrap icons in spans for sizing/color.
- Keep the dataset curated rather than exposing the full icondata_lu set.

## Testing
- Add focused tests if practical for fallback behavior; otherwise cover via manual verification.
