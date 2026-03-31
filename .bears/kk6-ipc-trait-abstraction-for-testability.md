---
id: kk6
title: IPC trait abstraction for testability
status: open
priority: P1
created: 2026-03-31T16:40:34.554823843Z
updated: 2026-03-31T16:40:40.269576847Z
tags:
- frontend
- refactor
depends_on:
- tfv
parent: 82y
---

## Summary
Abstract the IPC layer behind a trait so components can be tested with a mock backend. The real implementation calls Tauri `invoke`; tests provide a mock returning canned data.

## Acceptance Criteria
- [ ] `trait Ipc` (or similar) with async methods for all current IPC functions
- [ ] Real implementation (`TauriIpc`) wraps current `invoke` calls
- [ ] Mock implementation (`MockIpc`) returns configurable canned responses
- [ ] Trait object or concrete type injected via `provide_context` / `use_context`
- [ ] All existing component code uses the trait instead of calling `ipc::` functions directly
- [ ] App still compiles and runs without behavior change

## Implementation Notes
- Files: `src/app/ipc.rs` (refactor), new `src/app/ipc/mock.rs` (behind `#[cfg(test)]`)
- The trait needs to be object-safe or use an enum dispatch to avoid `dyn async`
- Consider using an enum `IpcBackend { Tauri(TauriIpc), Mock(MockIpc) }` with match dispatch
- This overlaps with task `xaz` (simplify IPC boilerplate) — consider doing together
- The `MockIpc` should use `RefCell<VecDeque<Result<T>>>` or similar for configurable responses

## Testing
- Existing app works unchanged
- A test can `provide_context(MockIpc::new(...))` and mount a component