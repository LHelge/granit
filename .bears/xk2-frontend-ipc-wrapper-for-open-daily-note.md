---
id: xk2
title: Frontend IPC wrapper for open_daily_note
status: done
priority: P1
created: 2026-04-03T23:10:18.164049150Z
updated: 2026-04-03T23:18:28.051591891Z
tags:
- frontend
- ipc
depends_on:
- cy7
parent: hh6
---

## Summary
Add an IPC wrapper in the frontend for the `open_daily_note` command.

## Acceptance Criteria
- [ ] `src/app/ipc.rs` gains `pub async fn open_daily_note() -> Result<Note, String>`
- [ ] Calls `invoke("open_daily_note", JsValue::NULL)` and deserializes the result

## Implementation Notes
- Follow the exact pattern of `read_note()` in `ipc.rs`
- Returns full `Note` (with `meta` and `html`)
- No arguments needed — backend reads config and date internally