---
id: m9t
title: Make filename the single source of truth for note titles
status: done
priority: P1
created: 2026-03-27T12:22:59.652856Z
updated: 2026-03-27T13:01:27.424041Z
parent: wem
---

## Summary

Unify note title semantics around the filename or slug so the app no longer derives titles from frontmatter or markdown headings. This keeps note identity simple, matches the planned nested-folder model with globally unique filenames, and avoids conflicting definitions of what a note is called.

## Acceptance Criteria

- [ ] The backend no longer derives note titles from frontmatter or `#` headings.
- [ ] `NoteMeta.title` either mirrors the filename-derived title or is removed if redundant.
- [ ] The frontend uses the filename-derived title consistently in note lists, editor chrome, and related UI.
- [ ] Any UI affordance that renames a note is clearly labeled as a filename change, not document metadata editing.
- [ ] Tests reflect the new title semantics.

## Implementation Notes

- Review src-tauri/src/cave/note.rs, src-tauri/src/cave/mod.rs, src/app/components/note_list.rs, and src/app/components/editor.rs.
- Revisit whether `title` should remain a separate field or become a derived duplicate of `slug` for display convenience.
- Coordinate this change with the nested folders epic so the identity model stays stable.

## Edge Cases

- Notes containing frontmatter `title` values.
- Notes with first headings that differ from the filename.
- Existing notes created under the old derived-title behavior.

## Testing

- Update backend tests around metadata extraction.
- Update frontend behavior checks where title or slug display assumptions change.
- cargo fmt && cargo clippy && cargo test must pass.
