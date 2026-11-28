---
id: zem.4q
title: Extract a Tauri-free core crate from src-tauri
status: done
priority: P1
created: "2026-11-26T21:08:54.928028Z"
updated: "2026-11-26T23:59:40.963954938Z"
tags:
  - gpui
  - port
  - backend
  - core
attempts: 1
---

## Summary

Move the backend logic (cave, markdown, agent, backup, config) out of `src-tauri/` into a new workspace crate with no Tauri dependency, so the GPUI app can link it without pulling in Tauri, and the Tauri app keeps working on top of it until the cutover.

## Acceptance Criteria

- [ ] A new crate (name to agree, e.g. `granit-core`) holds `cave/`, `markdown/`, `agent/`, `backup/` and the config types and loading that these need
- [ ] `cargo tree -p <core> -i tauri` finds nothing
- [ ] `src-tauri` depends on the core crate and still builds and passes its tests; `commands/`, `scheme.rs` and `lib.rs` stay in `src-tauri`
- [ ] The core crate's tests run with `cargo test -p <core>`; CI and the `pre-push` hook run them

## Implementation Notes

- The spike found only two Tauri touchpoints in the core modules (details in `baa.ng`, "Tauri coupling in the backend"):
  - `src-tauri/src/cave/error.rs`: `impl From<tauri::Error> for CaveError` feeds `CaveError::Window`, used by the presentation window. Keep the conversion in `src-tauri` (map the error at the call site) or drop the variant once presentations are gone.
  - `src-tauri/src/agent/vectordb.rs`: `tauri::async_runtime::spawn` for the debounced embedding update. Replace with a plain `tokio::spawn`, or take a spawner from the host.
- Tauri-plugin-backed behaviour (last-open cave path via `tauri-plugin-store`, dialogs, opener, clipboard, logging, updater) stays out of the core crate; the GPUI app gets its own versions in later tasks.
- Keep `thiserror` error enums per module; no `anyhow` (`CLAUDE.md`).
- `AppState` (`commands/state.rs`) is Tauri-managed glue; decide whether its locking helpers (`with_cave`, `ensure_agent`, generation counter) belong in the core crate as a host-agnostic state type, since the GPUI app needs the same behaviour.

## Edge Cases

- Feature flags or heavy dependencies (fastembed, rig) must not force Tauri back in through a transitive path.

## Testing

- Existing backend tests move with the code and pass unchanged
- `cargo fmt --all`, both clippy invocations and `cargo test` for both crates pass