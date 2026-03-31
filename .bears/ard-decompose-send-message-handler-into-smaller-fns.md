---
id: ard
title: Decompose send_message handler into smaller fns
status: done
priority: P2
created: 2026-03-31T16:33:19.412365316Z
updated: 2026-03-31T17:06:51.860734386Z
tags:
- backend
- refactor
parent: 4cm
---

## Summary
The `send_message` Tauri command in lib.rs spans ~40 lines with lock contention, manual stream iteration, history management, and event emission. Should be decomposed.

## Acceptance Criteria
- [ ] `send_message` handler is thin — delegates to methods on Agent
- [ ] Stream processing extracted to Agent impl method
- [ ] History append extracted to Agent impl method
- [ ] Event emission logic is clear and testable

## Implementation Notes
- Files: `src-tauri/src/lib.rs`, `src-tauri/src/agent/mod.rs`
- Move streaming logic to `impl Agent { async fn stream_response(...) }`
- Keep the Tauri command as a thin wrapper that gets state and delegates

## Testing
- Existing agent tests pass; behavior unchanged