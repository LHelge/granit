---
id: zem.ru
title: "Chat panel: attach notes and resize the panel"
status: done
priority: P2
created: "2026-11-27T01:03:35.685882225Z"
updated: "2026-11-27T06:05:22.004788927Z"
tags:
  - gpui
  - port
  - frontend
  - agent
depends_on:
  - zem.up
  - zem.4k
attempts: 1
---

## Summary

The GPUI chat panel (zem.up) left out two parts of the current chat: attaching notes to a message (the active note, other notes, and the selected text in the editor; `src/app/agent/mod.rs`, sent as `attached_notes` to `send_message`), and resizing the panel with a drag handle. Today `build_agent_prompt` is called with `&[]` in `granit-app/src/chat/mod.rs`.

## Acceptance Criteria

- [ ] Notes can be attached to a message as in the current app, including the active note and its selected text; they reach `build_agent_prompt`
- [ ] The chat panel can be resized by dragging; the width is saved in `agent_panel.width`

## Testing

- A `gpui::test` that an attached note reaches the prompt