---
id: 9ma
title: 'Chat bubbles: Use chat/chat-bubble for agent messages'
status: open
priority: P1
created: 2026-04-04T21:14:36.009271859Z
updated: 2026-04-04T21:14:36.009271859Z
tags:
- frontend
depends_on:
- fsw
parent: 9fd
---

## Summary

Replace hand-rolled message bubble styling in the agent panel with DaisyUI `chat` component classes. User messages go right (`chat-end`), assistant messages go left (`chat-start`).

## Acceptance Criteria

- [ ] User messages use `chat chat-end` wrapper with `chat-bubble` content
- [ ] Assistant messages use `chat chat-start` wrapper with `chat-bubble` content
- [ ] User bubbles use `chat-bubble-primary` or `chat-bubble-neutral` color
- [ ] Streaming content bubble uses `chat chat-start` with `chat-bubble`
- [ ] Prose/markdown rendering inside chat-bubble preserved
- [ ] Tool call items can remain as a separate non-chat display or use `chat-header`

## Implementation Notes

- DaisyUI chat syntax: `<div class="chat chat-start"><div class="chat-bubble">content</div></div>`
- `inner_html` with `prose` class should work inside `chat-bubble`
- May need `chat-bubble` + prose size overrides to keep compact sizing

## Files to Modify

- `src/app/components/agent_panel.rs` — message rendering (committed messages, streaming bubble, empty state)
