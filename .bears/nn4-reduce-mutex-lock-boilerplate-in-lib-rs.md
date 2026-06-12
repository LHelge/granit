---
id: nn4
title: Reduce mutex lock boilerplate in lib.rs
status: done
priority: P2
created: 2026-03-31T16:32:52.083174823Z
updated: 2026-03-31T17:05:26.008996863Z
tags:
- backend
- refactor
parent: 4cm
---

## Summary
Every Tauri command in lib.rs repeats `state.lock().map_err(|_| XError::Poisoned)?` (15+ times). This boilerplate obscures the actual logic.

## Acceptance Criteria
- [ ] Create extension trait or wrapper (e.g. `trait StateExt<T>` with `fn locked(&self) -> Result<MutexGuard<T>, E>`)
- [ ] Replace all manual lock patterns in lib.rs
- [ ] Consider also wrapping the `with_cave`/`with_cave_mut` helpers if they can be simplified

## Implementation Notes
- Files: `src-tauri/src/lib.rs`
- The trait could be generic over the error type: `fn locked<E: From<PoisonedError>>(&self) -> Result<Guard, E>`
- Alternative: wrapper struct `AppState` with typed accessors: `state.config()`, `state.cave()`, `state.agent()`

## Testing
- Compile + existing tests pass; no behavior change