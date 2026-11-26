---
id: zem.5c
title: Templates and daily notes
status: open
priority: P2
created: "2026-11-26T21:09:59.256361Z"
updated: "2026-11-26T21:09:59.256361Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4k
---

## Summary

Bring over the remaining note features that are not presentations or mermaid: creating notes from templates in `.granit/templates/`, editing templates, and the daily-note button with auto-create and open.

## Acceptance Criteria

- [ ] New note from a template, rendered through the core crate as today
- [ ] Templates can be listed, created, renamed, deleted and edited in the editor
- [ ] The daily-note button opens today's note, creating it from the configured template when missing
- [ ] Anything else in the current app that is neither presentations nor mermaid and has no task in this epic is listed here and either ported or explicitly dropped with the owner

## Implementation Notes

- Current code: template and daily-note commands in `src-tauri/src/commands/`, frontend in `src/app/`.
- Template editing: the editor is markdown-only; templates contain Tera syntax, which the spike's round-trip found lossless.

## Testing

- Core tests exist for template rendering; add `gpui::test`s for the daily-note flow