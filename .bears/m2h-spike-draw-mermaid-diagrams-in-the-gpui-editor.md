---
id: m2h
title: "Spike: draw mermaid diagrams in the GPUI editor"
status: open
priority: P3
created: "2026-11-28T11:21:15.104030067Z"
updated: "2026-11-28T11:21:15.104030067Z"
tags:
  - gpui
  - spike
  - markdown
---

## Summary

The GPUI app shows a fenced `mermaid` block as a code block (decision in `zem.t7`, 2026-11-28). Find out whether diagrams can come back without Node: zorite-editor has `EditorState::set_block_mermaid_provider`, which takes a function from the block's source to a rendered bitmap and its logical size, rendered and cached by the host off the main thread.

## Acceptance Criteria

- [ ] Rust mermaid renderers evaluated: which diagram types they draw, quality against mermaid.js on a few real notes, dependency weight, maintenance
- [ ] A recommendation: adopt one (with an implementation task), wait, or drop diagrams for good
- [ ] If adopted: how theme colors and the light/dark switch reach the renderer

## Implementation Notes

- Nothing mermaid-related is in the dependency tree today; no renderer has been evaluated yet.
- The editor shows the raw source while the caret is inside the block or the diagram is not rendered yet.

## Testing

- None; a spike