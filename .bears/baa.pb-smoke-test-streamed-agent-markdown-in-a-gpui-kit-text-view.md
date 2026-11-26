---
id: baa.pb
title: Smoke-test streamed agent markdown in a GPUI Kit text view
status: open
priority: P2
created: "2026-11-26T12:16:03.395914Z"
updated: "2026-11-26T12:16:03.395914Z"
tags:
  - spike
  - gpui
  - agent
depends_on:
  - baa.7f
---

## Summary

Gate G6. After the note editor, agent chat is the largest surface that relies on HTML today (`inner_html` in `src/app/agent/messages.rs`). Check that GPUI Kit's markdown view (`TextView`) can render a streamed response well enough, without wiring the real agent.

## Acceptance Criteria

- [ ] A throwaway panel replays a canned markdown response in small chunks on a timer (about 30 chunks per second) into a GPUI Kit `TextView`
- [ ] The response contains: headings, a nested list, a task list, a table, a fenced code block with a language, inline code, a link, and a `[[wiki-link]]`
- [ ] Rendering stays smooth while streaming, with no visible flicker or full re-layout jumps, and the view can follow the bottom
- [ ] Text can be selected and copied across blocks
- [ ] Link clicks are delivered to host code; recorded whether `[[wiki-link]]` can be made clickable (via a markdown extension/plugin, or by pre-rewriting to a normal link as `Markdown` rendering does today)
- [ ] Gaps listed (task lists and raw HTML handling are unconfirmed in the docs)

## Implementation Notes

- `TextView` supports incremental markdown parsing, tables, math blocks, link-click handling and markdown extensions as of GPUI Kit 0.6-0.11; check gpui-kit.com for the current API.
- Do not depend on `rig` or the real `Agent` here. A hard-coded string split into chunks is enough.
- This task is independent of the cave and editor tasks and can run in parallel with them.

## Testing

- Manual; note results for the write-up