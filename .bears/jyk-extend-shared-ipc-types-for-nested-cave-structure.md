---
id: jyk
title: Extend shared IPC types for nested cave structure
status: done
priority: P2
created: 2026-03-27T12:42:05.294299Z
updated: 2026-03-30T12:30:54.007489Z
depends_on:
- d2a
parent: ph5
---

## Summary

Extend the shared IPC contract so nested note and folder information can be represented cleanly across backend and frontend boundaries. This task should build on the shared IPC crate and the finalized backend nested-folder model.

## Acceptance Criteria

- [ ] Shared IPC types represent nested note and folder structure clearly.
- [ ] Path-like data is encoded consistently and appropriately for the IPC boundary.
- [ ] The frontend and backend compile against the updated shared types.
- [ ] The shared types match the agreed nested-folder identity model.

## Implementation Notes

- Build on the shared IPC crate introduced by the review follow-up work.
- Keep the shared types transport-focused and free of UI-specific concerns.
- Coordinate closely with the backend path-aware operations task.

## Edge Cases

- Root-level notes and folders versus nested ones.
- Representing folder nodes with no child notes.
- Distinguishing filename identity from relative path location.

## Testing

- Verify frontend and backend compilation against the updated shared contract.
- cargo fmt && cargo clippy && cargo test must pass.
