---
id: 8b3
title: Build nested cave explorer and note workflows
status: done
priority: P2
created: 2026-03-27T12:42:05.377809Z
updated: 2026-03-30T12:45:40.471705Z
depends_on:
- d2a
- jyk
parent: ph5
---

## Summary

Update the frontend so users can browse nested folders and work with notes in the resulting tree structure without regressing current editing workflows. This task should consume the finalized backend behavior and shared IPC types rather than inventing its own structure.

## Acceptance Criteria

- [ ] The frontend can display nested cave structure clearly.
- [ ] Selecting and opening notes works for both root and nested notes.
- [ ] Nested-folder workflows integrate cleanly with the existing editor and sidebar UX.
- [ ] Duplicate-name and path-related errors are surfaced clearly where relevant.
- [ ] The UI remains usable on current desktop layouts.

## Implementation Notes

- Review src/app/components/sidebar.rs, src/app/components/note_list.rs, and related state flow before implementation.
- Reuse the shared IPC contract rather than rebuilding nested structures on the client.
- Keep the first iteration pragmatic; do not overbuild tree features that are not needed yet.

## Edge Cases

- Empty folders.
- Deeply nested notes.
- Selecting a note after folder structure changes.
- Creating or moving notes into nested folders if those workflows are included.

## Testing

- Manual verification of the main nested-folder workflows is required.
- Add test coverage if logic is extracted into testable units.
