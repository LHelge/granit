---
id: dfs
title: Frontend refresh on agent tool mutations
status: done
priority: P2
created: 2026-03-31T21:58:00.273154654Z
updated: 2026-04-01T13:53:52.538123Z
tags:
- frontend
- backend
depends_on:
- 8fv
parent: qk2
---

## Summary

When the agent creates, edits, or moves notes, the frontend's file tree and editor may be stale. Add event-based notifications so the frontend refreshes.

## Acceptance Criteria

- [ ] Agent tools emit Tauri events after mutations (e.g., `cave:notes-changed`, `cave:note-edited`)
- [ ] Frontend listens for these events and refreshes the note list / folder tree
- [ ] If the active note was edited by the agent, the editor content refreshes
- [ ] No double-refresh on normal user saves (only agent tool mutations trigger these events)
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Tools need `AppHandle` (or a clone) to emit events — pass it during construction or via a shared `Arc<AppHandle>`
- Alternatively, tools can return a "side effect" marker and the `send_message` handler emits events after tool execution — but rig's streaming model may not support this easily
- Simplest approach: tools hold `Arc<AppHandle>` and emit directly after cave mutations
- Frontend: add event listeners in the sidebar/tree components similar to the agent streaming listeners