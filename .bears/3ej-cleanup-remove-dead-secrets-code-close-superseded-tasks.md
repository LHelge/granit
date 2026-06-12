---
id: 3ej
title: 'Cleanup: remove dead secrets code, close superseded tasks'
status: done
priority: P2
created: 2026-03-31T21:51:24.791998852Z
updated: 2026-04-03T23:36:12.647355491Z
tags:
- cleanup
depends_on:
- as6
- yh6
parent: c56
---

## Summary

Final cleanup pass: remove any remaining dead code from the old secrets system, cancel the superseded `xd5` epic tasks, and ensure everything compiles and tests pass cleanly.

## Acceptance Criteria

- [ ] No references to `secrets.env`, `Secrets`, `get_secret`, `set_secret`, `dotenvy` remain in codebase
- [ ] No unused imports or dead code warnings from clippy
- [ ] Superseded `xd5` epic tasks are cancelled/closed with a note pointing to this epic
- [ ] `cargo fmt && cargo clippy && cargo test -p granit && cargo test -p granit-types` all pass
- [ ] README updated if it references secrets.env setup

## Implementation Notes

- Use `cargo clippy` to find dead code
- `grep -r "secret" src-tauri/src/` to find any remaining references
- Check `src/app/ipc.rs` for leftover secret-related IPC wrappers