---
id: wyh
title: Frontend IPC and event listeners for chat streaming
status: open
priority: P1
created: 2026-03-27T21:33:53.343014628Z
updated: 2026-03-27T21:33:53.343014628Z
tags:
- frontend
depends_on:
- kjn
parent: ann
---

## Summary

Add IPC wrappers and Tauri event listeners in the frontend to send messages and receive streaming responses. Define the shared types for chat messages.

## Acceptance Criteria

- [ ] `ChatMessage` type in `granit-types`: `{ role: "user"|"assistant", content: String }`
- [ ] `ipc::send_message(message: &str)` wrapper that invokes the Tauri command
- [ ] Frontend listens for `agent:stream-chunk`, `agent:stream-done`, `agent:stream-error` Tauri events
- [ ] Event listeners update reactive signals that the `AgentPanel` can consume
- [ ] Event listener cleanup on component unmount

## Implementation Notes

- Files: `granit-types/src/lib.rs`, `src/app/ipc.rs`
- Use `window.__TAURI__.event.listen("agent:stream-chunk", callback)` via `wasm_bindgen`
- Or use the `tauri-plugin-event` JS API — check what's available in Tauri 2
- The listener should append text deltas to a `streaming_content: RwSignal<String>`
- On `stream-done`: finalize the message into the chat history

## Testing

- Manual: verify events are received in browser console