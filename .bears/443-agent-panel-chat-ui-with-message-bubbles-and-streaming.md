---
id: '443'
title: Agent panel chat UI with message bubbles and streaming
status: open
priority: P1
created: 2026-03-27T21:34:06.144233731Z
updated: 2026-03-27T21:34:06.144233731Z
tags:
- frontend
depends_on:
- wyh
parent: ann
---

## Summary

Build out the `AgentPanel` component to display a proper chat UI: message list with user/assistant bubbles, streaming indicator, and input handling.

## Acceptance Criteria

- [ ] Chat history displayed as a scrollable message list
- [ ] User messages styled distinctly from assistant messages (e.g., alignment or color)
- [ ] Streaming response shows progressively as tokens arrive (text appended in real-time)
- [ ] Typing/streaming indicator visible while response is in progress
- [ ] Auto-scroll to bottom on new messages and during streaming
- [ ] Input disabled while streaming (or sends are queued)
- [ ] Empty state: friendly placeholder when no messages yet
- [ ] Error messages displayed inline in the chat (e.g., "Could not connect to Ollama")

## Implementation Notes

- Files: `src/app/components/agent_panel.rs`
- Signals: `messages: RwSignal<Vec<ChatMessage>>`, `streaming_content: RwSignal<String>`, `is_streaming: RwSignal<bool>`
- Use Tailwind classes consistent with the existing dark theme (stone-* palette)
- User messages: right-aligned or distinct background (e.g., `bg-stone-700`)
- Assistant messages: left-aligned, default background
- Streaming: show `streaming_content` at the bottom of the message list with a cursor/blinking indicator

## Edge Cases

- Very long messages: ensure wrapping and scroll work
- Rapid messages: handle state correctly
- Panel resize: messages remain visible