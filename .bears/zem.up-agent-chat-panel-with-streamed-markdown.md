---
id: zem.up
title: Agent chat panel with streamed markdown
status: open
priority: P1
created: "2026-11-26T21:09:38.859735Z"
updated: "2026-11-26T21:09:38.859735Z"
tags:
  - gpui
  - port
  - frontend
  - agent
depends_on:
  - zem.4b
---

## Summary

Rebuild the agent chat on GPUI Kit's `TextView`, wired to the real `Agent` in the core crate: streamed responses, tool calls and results, Ask/Agent modes, provider and model selection, and conversation history. The spike showed streaming renders formatted markdown while it arrives, which the current app does not.

## Acceptance Criteria

- [ ] Sending a message streams the reply through `Agent::stream_with_history`, rendered incrementally in a `TextView` that follows the bottom
- [ ] Tool calls and tool results are shown as today
- [ ] Ask and Agent modes, provider and model pickers; changing them mid-stream tears down and rebuilds the agent as the backend does now
- [ ] `[[wiki-links]]` in replies are clickable and open the note; the rewriter skips code spans and fenced code (the spike's rewriter did not)
- [ ] Task-list checkboxes in agent output are disabled, as today
- [ ] Text can be selected and copied across blocks
- [ ] Cave mutations made by agent tools refresh the explorer and the open note

## Implementation Notes

- Spike smoke test: `granit-gpui/src/bin/stream_smoke.rs` on `spike/gpui`; findings in `baa.pb` (incremental parse, `FollowMode::Tail`, `on_link_click`, raw HTML handling, no `{#id}` support).
- Current behaviour: `src/app/agent/` (messages, input, mode switch) and `src-tauri/src/commands/agent.rs`.
- Streaming runs on the async runtime in the core crate; bridge items to the GPUI foreground with an entity update per chunk.
- Broken links in chat cannot be styled differently (the `.link()` hook can only re-target or disable); decide whether to disable them.

## Testing

- A test that feeds a canned `AgentStream` and checks the rendered blocks and link targets; no network