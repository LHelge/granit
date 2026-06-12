---
id: ph5
title: Nested folders with globally unique filenames
type: epic
status: done
priority: P1
created: 2026-03-27T12:19:07.613106Z
updated: 2026-03-30T12:47:57.665614Z
---

## Scope

Plan and implement near-term support for nested folders inside a cave while preserving filename-derived titles and preventing the same filename from existing in multiple folders. This epic captures the product and architecture direction only; the implementation task breakdown will be added later.

## Product Decisions

- Nested folders are a planned near-term feature.
- Filenames must remain globally unique across the entire cave, even when notes live in different subfolders.
- The filename/slug is the single source of truth for a note title.
- Frontmatter and markdown headings must not override the note title.
- The displayed title should continue to reflect the filename-derived note identity rather than introduce duplicate-name disambiguation in the UI.

## Acceptance Criteria

- [ ] Cave operations support notes stored in nested subfolders.
- [ ] The system prevents duplicate filenames anywhere in the cave tree.
- [ ] The note identity model is clear and consistent across backend storage, frontend state, and future wiki-link behavior.
- [ ] The title model is clear and consistent: filenames define titles, and note content does not.
- [ ] The UI can browse or represent nested notes without regressing current note workflows.
- [ ] Documentation reflects the chosen nested-folder and filename-uniqueness rules.

## Implementation Notes

- Revisit the current flat-file assumptions in src-tauri/src/cave/mod.rs and src-tauri/src/cave/note.rs.
- Decide whether note identity should be relative path, filename, or a separate internal identifier while preserving globally unique filenames.
- Remove or redesign any content-derived title extraction that conflicts with filename-defined titles.
- Revisit list and selection behavior in the frontend once the backend model is defined.
- Defer sub-task creation until the implementation approach is designed.
