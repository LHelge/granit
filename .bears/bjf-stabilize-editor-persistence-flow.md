---
id: bjf
title: Stabilize editor persistence flow
status: done
priority: P1
created: 2026-03-27T12:11:19.259747Z
updated: 2026-03-27T13:09:13.535557Z
depends_on:
- c2z
- m9t
parent: wem
---

## Summary

Make note saving and renaming reliable so the editor cannot silently lose user changes when switching notes or saving after a title change. The current frontend orchestration performs a rename and a content save as separate IPC calls and ignores failures in the autosave path.

## Acceptance Criteria

- [ ] Note switching no longer silently drops rename or save failures.
- [ ] The persistence path for title changes and content changes is coordinated through a safer backend-facing API or equivalent single logical operation.
- [ ] The frontend surfaces save failures to the user instead of degrading into empty or stale state.
- [ ] The editor state stays consistent after failed rename/save attempts.
- [ ] Tests cover the critical failure paths for rename-plus-save behavior.

## Implementation Notes

- Review the current autosave and save flows in src/app/components/editor.rs.
- Consider replacing the current two-step frontend choreography with a dedicated backend command that handles title and content persistence together.
- Revisit whether note-switch autosave should remain enabled immediately or whether explicit save plus dirty-state indication is safer for the near term.
- Keep the handler thin in src-tauri/src/lib.rs and delegate logic into cave or a dedicated editor persistence module if the workflow grows.

## Edge Cases

- Rename succeeds but content write fails.
- Save is attempted while another save is in flight.
- User switches notes with unsaved title/content changes.
- Empty or invalid titles.

## Testing

- Add or update backend tests for rename/save failure scenarios where practical.
- Add test coverage around the frontend editor flow if the logic can be extracted into testable units.
- cargo fmt && cargo clippy && cargo test must pass.
