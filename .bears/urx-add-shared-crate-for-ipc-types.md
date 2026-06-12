---
id: urx
title: Add shared crate for IPC types
status: done
priority: P2
created: 2026-03-27T12:29:24.793966Z
updated: 2026-03-27T14:58:27.155188Z
depends_on:
- 3s4
- m9t
parent: wem
---

## Summary

Extract the serializable data types shared between the frontend and backend into a dedicated Rust crate used by both sides. This provides a stronger compile-time contract at the IPC boundary and reduces the risk of drift between duplicated type definitions.

## Acceptance Criteria

- [ ] Shared IPC-facing types live in a dedicated crate consumed by both the frontend and backend.
- [ ] The frontend no longer maintains duplicate local definitions for types that can come from the shared crate.
- [ ] The shared crate remains lightweight and suitable for both WASM and native targets.
- [ ] Cargo workspace configuration reflects the new crate cleanly.
- [ ] Existing IPC flows continue to compile and behave correctly with the shared types.

## Implementation Notes

- Review the current duplicated types in src/app/types.rs and src-tauri/src/config/mod.rs plus cave note types returned over IPC.
- Keep the shared crate free of Tauri, Leptos, and platform-specific dependencies.
- Prefer serde-only domain and transport types.
- Coordinate with current-cave-state and title-semantics tasks so the shared contract does not need immediate rework.

## Edge Cases

- Types used only on one side should not be moved just for symmetry.
- Avoid introducing dependencies that make the crate unusable from the WASM frontend.
- Be explicit about whether path-like data is represented as strings or path types across the IPC boundary.

## Testing

- Verify both frontend and backend compile against the shared crate.
- cargo fmt && cargo clippy && cargo test must pass.
