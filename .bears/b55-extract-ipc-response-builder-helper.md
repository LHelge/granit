---
id: b55
title: Extract IPC response builder helper
status: done
priority: P1
created: 2026-04-04T21:40:46.864493867Z
updated: 2026-04-04T22:59:27.464639772Z
tags:
- duplication
- backend
depends_on:
- eup
parent: drc
---

## Summary
The 5-line "build IPC config + set active_cave" block is repeated 6+ times in lib.rs command handlers.

## What to do
- Extract: `fn ipc_response(config: &AppConfig, state: &AppState) -> Result<IpcConfig, ConfigError>`
- Replace all 6+ occurrences with a call to the helper

## Files
- `src-tauri/src/lib.rs`