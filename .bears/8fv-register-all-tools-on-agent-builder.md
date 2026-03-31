---
id: 8fv
title: Register all tools on agent builder
status: open
priority: P1
created: 2026-03-31T21:57:47.696358322Z
updated: 2026-03-31T21:57:47.696358322Z
tags:
- backend
depends_on:
- 8fz
- 6ry
parent: qk2
---

## Summary

Wire all implemented tools into the agent builder so they are available when the agent processes messages. Update the `Agent::from_config()` (or the new `from_provider_config`) method to accept shared state and register tools.

## Acceptance Criteria

- [ ] `Agent` builder accepts `Arc<Mutex<Option<Cave>>>` and `Arc<Mutex<Option<String>>>` (active note)
- [ ] All 7 tools registered via `.tool(ToolInstance { cave: arc.clone(), ... })` on the rig agent builder
- [ ] Each `ProviderAgent` variant (Ollama, Anthropic, Mistral) registers the same tools
- [ ] Agent rebuild (`reset_agent`) passes the shared state to the new agent
- [ ] Tools work end-to-end: user can ask "read my note about X" and the agent calls the tool
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/agent/mod.rs`, `src-tauri/src/lib.rs`
- The rig agent builder pattern: `client.agent(model).preamble(...).tool(tool1).tool(tool2).build()`
- Each `.tool()` call changes the generic type of the builder — this may require adjusting `ProviderAgent` to use a single concrete toolset type or boxing
- If the rig builder's type becomes unwieldy, consider a helper that takes a builder and adds all tools
- The system prompt should mention available tools so the LLM knows to use them
- `send_message` currently snapshots the agent — tools hold Arc so they survive the clone