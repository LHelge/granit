---
id: 5qp
title: Replace message tuple with named struct
status: open
priority: P3
created: 2026-03-31T16:33:41.283867095Z
updated: 2026-03-31T16:33:41.283867095Z
tags:
- frontend
- refactor
parent: 4cm
---

## Summary
In agent_panel.rs, messages are stored as `(ChatMessage, Option<String>)` tuples. The meaning of the `Option<String>` (rendered HTML) is unclear without reading surrounding code.

## Acceptance Criteria
- [ ] Replace tuple with a named struct (e.g. `struct DisplayMessage { message: ChatMessage, rendered_html: Option<String> }`)
- [ ] Update all usage sites

## Implementation Notes
- Files: `src/app/components/agent_panel.rs`
- Simple find-and-replace refactor