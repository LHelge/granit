---
id: yes
title: Bound agent message history
status: open
priority: P1
created: 2026-03-31T16:32:23.230839413Z
updated: 2026-03-31T16:32:23.230839413Z
tags:
- backend
parent: 4cm
---

## Summary
`Agent.history: Vec<Message>` grows without bound. Long chat sessions will consume excessive memory and eventually exceed the LLM's context window.

## Acceptance Criteria
- [ ] Add configurable max history limit (default: e.g. 100 messages)
- [ ] When limit exceeded, drop oldest messages (sliding window)
- [ ] Config field in `AgentConfig` for `max_history`

## Implementation Notes
- Files: `src-tauri/src/agent/mod.rs`, `granit-types/src/config.rs`
- Use `VecDeque` instead of `Vec` for O(1) front removal
- Add `max_history: usize` to `AgentConfig` with a sensible default

## Testing
- Unit test: push 200 messages with limit 100, verify only last 100 remain