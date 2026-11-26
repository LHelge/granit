---
id: baa.pb
title: Smoke-test streamed agent markdown in a GPUI Kit text view
status: done
priority: P2
created: "2026-11-26T12:16:03.395914Z"
updated: "2026-11-26T21:06:19.021248Z"
tags:
  - spike
  - gpui
  - agent
depends_on:
  - baa.7f
attempts: 1
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
## Findings so far (2026-11-26, gpui-kit 0.10.0)

Smoke test: `cargo run -p granit-gpui --bin stream_smoke` (`GRANIT_SMOKE_NO_FADE=1` disables the Kit's fade-in, `GRANIT_SMOKE_EXIT=1` quits after the replay). Headless run: 2236 chars in 12.3 s, no late ticks, no panic. Everything visual is still unverified and needs the owner's eyes.

- Streaming: `TextViewState::push_str` queues the delta to a background parse that re-parses only the last block, not the document (read from source, not measured). A long table or code block still streaming is re-parsed and re-highlighted whole per chunk.
- Follow-bottom: `ListState::set_follow_mode(FollowMode::Tail)`; scrolling up pauses, scrolling back resumes.
- Wiki-links: pre-rewriting `[[t]]` / `[[t|label]]` to `[label](<wiki:t>)` on the stream works (unit-tested for every chunk size) and reaches `on_link_click`. The spike rewriter is naive about code spans. The plugin route maps one syntax node to one atomic inline element and has no inline-syntax extension point, so it looks poor (not tried).
- Broken links: the per-draw `.link()` hook can re-target or disable a link, not style it differently.
- Task lists: rendered as checkboxes, interactive only when `on_task_toggle` is set.
- Raw HTML: on by default through the Kit's limited HTML parser, nothing executes; `markdown_raw_html(false)` shows it literally.
- Heading anchors `{#id}`: no native support.

## Owner verdict (2026-11-26)

The streaming "looks awesome": markdown formatting is applied while the response streams, unlike the current Tauri app, where it appears only once the response is done. Closed; G6 passes. The gaps above (naive wiki-link rewriting in code spans, no broken-link styling, no `{#id}` anchors) are port work, not blockers.
