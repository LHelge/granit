---
id: nst
title: Remove panic-prone backend state handling
status: open
priority: P2
created: 2026-03-27T12:34:57.155741Z
updated: 2026-03-27T12:39:19.207718Z
depends_on:
- 3s4
parent: wem
---

## Summary

Harden backend command and state handling so normal error paths do not panic the application. The current command layer uses `unwrap()` on shared-state locks and similar operations, which is simple but not the failure behavior we want in a desktop app.

## Acceptance Criteria

- [ ] Backend command and state access paths no longer rely on panic-prone `unwrap()` behavior where ordinary error handling is feasible.
- [ ] Lock acquisition or other state-access failures are mapped into explicit application errors.
- [ ] The resulting error behavior is consistent with the broader IPC error handling direction.
- [ ] The code remains simple and does not introduce unnecessary abstraction.

## Implementation Notes

- Review src-tauri/src/lib.rs first, especially command handlers and helpers that access shared state.
- Prefer explicit error mapping over hidden fallback behavior.
- Coordinate with the current-cave-state task if those changes touch the same code paths.

## Edge Cases

- Poisoned mutexes.
- State access failures during cave open or command execution.
- Avoid turning clear failures into silent no-ops.

## Testing

- Add focused backend tests where practical.
- cargo fmt && cargo clippy && cargo test must pass.
