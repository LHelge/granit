---
id: dpx
title: Refactor Cave state to Arc<Mutex> for tool sharing
status: open
priority: P1
created: 2026-03-31T21:56:50.973525941Z
updated: 2026-03-31T21:56:50.973525941Z
tags:
- backend
- refactor
parent: qk2
---

## Summary

Refactor `AppState` so the `Cave` is wrapped in `Arc<Mutex<>>` instead of a plain `Mutex`. This lets agent tools hold a cheap `Arc<Mutex<Cave>>` clone without borrowing `AppState`.

## Acceptance Criteria

- [ ] `AppState.cave` is `Arc<Mutex<Option<Cave>>>` instead of `Mutex<Option<Cave>>`
- [ ] All existing `with_cave` / `with_cave_mut` / `lock_cave` usages still compile and work
- [ ] `AppState` exposes a `cave_arc() -> Arc<Mutex<Option<Cave>>>` method for agent tool construction
- [ ] No behavior change — purely a refactor
- [ ] `cargo test -p granit` passes

## Implementation Notes

- File: `src-tauri/src/lib.rs`
- Minimal change: wrap in `Arc::new(Mutex::new(...))`, update `lock_cave` to `.lock()` on the inner `Mutex`
- The `Arc` clone will be passed to tools during agent build