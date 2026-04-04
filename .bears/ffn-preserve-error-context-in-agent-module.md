---
id: ffn
title: Preserve error context in agent module
status: done
priority: P1
created: 2026-04-04T21:40:39.644221127Z
updated: 2026-04-04T22:37:44.096863111Z
tags:
- errors
- backend
parent: drc
---

## Summary
`ToolError::Cave(String)` and `AgentError::Stream(String)` lose the original error type via `.to_string()`. This makes debugging harder and prevents programmatic error matching.

## What to do
- Change `ToolError::Cave(String)` → `ToolError::Cave(#[from] crate::cave::CaveError)`
- Consider whether `AgentError::Stream(String)` can preserve more context (at minimum use `format!("{:?}", e)` for Debug output)
- Update `with_cave()` and `with_cave_mut()` helpers in agent/tools/mod.rs

## Files
- `src-tauri/src/agent/tools/mod.rs` — ToolError, with_cave helpers
- `src-tauri/src/agent/error.rs` — AgentError::Stream