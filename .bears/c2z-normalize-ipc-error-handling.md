---
id: c2z
title: Normalize IPC error handling
status: done
priority: P1
created: 2026-03-27T12:13:42.676156Z
updated: 2026-03-27T12:55:51.682936Z
parent: wem
---

## Summary

Make the frontend IPC layer return explicit success or failure information so the UI can distinguish backend errors from empty state, cancellation, or missing data. The current helpers collapse many failures into `None` or empty collections, which hides bugs and makes the main user flows unreliable.

## Acceptance Criteria

- [ ] Frontend IPC helpers use explicit `Result` types for command-backed operations.
- [ ] `Option` is only used where absence is a real business case, such as dialog cancellation.
- [ ] Startup, cave opening, note creation, note loading, and note listing can surface user-visible errors.
- [ ] Empty state and load failure are represented distinctly in the UI.
- [ ] The revised IPC layer does not introduce redundant error conversion logic across components.

## Implementation Notes

- Review src/app/ipc.rs and normalize the command helper return types.
- Update call sites in src/app/mod.rs, src/app/components/sidebar.rs, src/app/components/cave_selector.rs, and src/app/components/editor.rs.
- Keep the backend error payloads simple unless there is a strong need for richer structured errors.
- Consider adding a lightweight shared UI error pattern rather than ad hoc per-component banners.

## Edge Cases

- Startup config load fails.
- Cave open fails after a user picks a folder.
- Note list fetch fails versus succeeds with zero notes.
- Dialog cancellation should remain non-error behavior.

## Testing

- Add coverage for helper-level error conversion where practical.
- Verify the main UI flows continue to work with the new return types.
- cargo fmt && cargo clippy && cargo test must pass.
