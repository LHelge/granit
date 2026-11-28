---
id: zem.5c
title: Templates and daily notes
status: done
priority: P2
created: "2026-11-26T21:09:59.256361Z"
updated: "2026-11-27T02:05:41.777069767Z"
tags:
  - gpui
  - port
  - frontend
depends_on:
  - zem.4k
attempts: 1
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

## Inventory

What the current app has beyond presentations, mermaid and the features with their own tasks (zem.33, zem.ru, zem.hs, zem.8j, zem.kq, zem.nb), from `src/app/` and `src-tauri/src/commands/`.

Ported with this task:

- The template list's "Template Parameters" help (the Tera variables), in the Templates tab
- Template icons in the template list

Filed:

- zem.7j (note header): favorite toggle, icon picker, tags editor, created/modified dates and word count, rename by editing the title, backlinks section; also a template's icon and tags
- zem.ub (sidebar): show/hide toggle, resize handle, note count footer, empty-space context menu on the tree
- zem.th (editor): find button, find next/previous keys, find and replace, select next occurrence, `> [!` alert completion, copy note as HTML
- zem.s9 (dialogs and pickers): keyboard-shortcuts dialog, about dialog, font picker with search and preview
- zem.vz (agent files): skills and tasks tab (create, edit, rename, delete), Tera variables help for agent files

Not filed, left to the owner:

- Themes beyond Catppuccin light and dark (the epic decided light/dark only)
- Following a renamed template in `daily_note_template_slug` (neither app does it)
- Tera highlighting and completion while editing agent files