---
id: 6rf
title: Build searchable icon picker in frontmatter editor
status: open
priority: P1
created: 2026-04-03T14:10:39.361014027Z
updated: 2026-04-03T14:10:39.361014027Z
tags:
- frontend
- editor
- ux
depends_on:
- 6uq
- cdq
parent: gcy
---

## Summary

Add editor state and a searchable icon picker to the frontmatter UI so users can choose or clear a note icon from a curated set using a search field above a dense scrollable icon grid.

## Acceptance Criteria
- [ ] Editor state tracks the selected optional icon and initializes from note frontmatter.
- [ ] The frontmatter UI includes an icon trigger with current preview and label.
- [ ] Clicking the trigger opens an anchored popover with outside-click dismissal.
- [ ] The popover contains a search field above a scrollable icon grid.
- [ ] Grid filtering matches icon labels and custom search tags.
- [ ] Selecting an icon updates editor state and closes the popover.
- [ ] Users can clear the selection and return to the default icon.
- [ ] Saving and autosave pass the icon through existing note update flows.
- [ ] Empty searches show an explicit `No matching icons` state.

## Implementation Notes
- Extend `src/app/components/editor/mod.rs` and `src/app/components/editor/frontmatter.rs`.
- Reuse the backdrop-assisted popover structure from the font picker, adapted to a grid layout.
- Keep the picker anchored in the frontmatter area rather than expanding inline.

## Testing
- Verify search, selection, clear, reopen, and persistence behavior manually if no frontend component tests are in place.
