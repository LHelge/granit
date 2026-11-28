---
id: zem.t7
title: Decide the future of mermaid diagrams and presentations
status: done
priority: P3
created: "2026-11-26T21:10:30.037291Z"
updated: "2026-11-28T11:21:15.087345588Z"
tags:
  - gpui
  - port
  - decision
---

## Summary

The spike decided to drop mermaid diagrams and presentations from the new frontend, to be rebuilt later if needed. Before the cutover removes them, decide with the owner what users of existing caves get: nothing, a fallback, or a rebuilt feature.

## Acceptance Criteria

- [ ] Decision recorded: for mermaid (fenced `mermaid` blocks) and for presentations (`.granit/presentations/` templates, the `presentation` frontmatter field, the `granit://` presentation route)
- [ ] If dropped: what an existing note shows (a mermaid block as a code block; presentation frontmatter kept untouched on save), and the docs pages that go
- [ ] If kept: a follow-up task or epic for the rebuild; options to weigh include zorite-editor's `set_block_mermaid_provider`, and opening presentations in the system browser from a local server

## Implementation Notes

- Current implementation: `src-tauri/src/markdown/presentation.rs`, `src-tauri/src/commands/presentation.rs`, `src-tauri/src/cave/presentations.rs`, `js/mermaid.ts`, and the public DOM contract documented on the docs site.

## Testing

- None; a decision task
## Decision (owner, 2026-11-28)

- **Mermaid: dropped at the cutover, with a spike later.** A fenced `mermaid` block shows as a code block. Follow-up task: evaluate a Rust mermaid renderer behind zorite-editor's `set_block_mermaid_provider` (it wants a bitmap and its logical size).
- **Presentations: dropped from the GPUI app, maybe later.** The `presentation` frontmatter field and `.granit/presentations/` are left untouched on disk (the core never rewrites frontmatter it does not edit), and the rendering code stays in `granit-core`. Follow-up task: open the rendered slide page in the system browser from a temporary folder (templates keep working; no reload on save, no mermaid on slides).
- **Docs that go** (in `zem.sp`): `presentations.md` with the template DOM contract, and whatever the other pages say about mermaid diagrams.
