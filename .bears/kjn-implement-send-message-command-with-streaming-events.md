---
id: kjn
title: Implement send_message command with streaming events
status: done
priority: P1
created: 2026-03-27T21:33:42.039347935Z
updated: 2026-03-28T22:58:37.034482269Z
tags:
- backend
depends_on:
- nbp
parent: ann
---

## Summary

Add a `send_message` Tauri command that takes a user message and conversation history, calls the Ollama agent via rig-core's `StreamingChat` trait, and streams tokens back to the frontend via Tauri events.

## Acceptance Criteria

- [ ] `send_message` Tauri command: takes user message string, returns immediately (async)
- [ ] Agent state (`AgentState`) stored in `tauri::State` with conversation history
- [ ] Streaming: use `rig::streaming::StreamingPrompt` or `StreamingChat` to get token-by-token responses
- [ ] Each token chunk emitted as a Tauri event (`agent:stream-chunk`) with the text delta
- [ ] Stream completion emitted as `agent:stream-done` event
- [ ] Errors emitted as `agent:stream-error` event (Ollama not running, model not found, etc.)
- [ ] Conversation history preserved across messages within the session

## Implementation Notes

- Files: `src-tauri/src/agent/mod.rs`, `src-tauri/src/lib.rs`
- Use `app.emit("agent:stream-chunk", payload)` for Tauri events
- The command needs `tauri::AppHandle` to emit events and `tauri::State<AgentState>` for the agent
- rig-core streaming returns a `Stream` — iterate with `while let Some(chunk) = stream.next().await`
- Keep conversation history in `AgentState` as a `Vec<Message>` (or rig-core's chat history type)

## Edge Cases

- Ollama not running → emit error event with helpful message
- Model not found → emit error event
- User sends message while previous response is streaming → queue or reject

## Testing

- Unit test for conversation history management
- Manual test: verify streaming events arrive in the frontend console