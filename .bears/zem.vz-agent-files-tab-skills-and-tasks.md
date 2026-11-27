---
id: zem.vz
title: "Agent files tab: skills and tasks"
status: open
priority: P2
created: "2026-11-27T02:06:51.119029352Z"
updated: "2026-11-27T02:06:51.119029352Z"
tags:
  - gpui
  - port
  - frontend
  - agent
depends_on:
  - zem.4k
---

## Summary

The current app has a sidebar tab to create, edit, rename and delete agent skills (`.granit/agent/skills/<name>/SKILL.md`, with description) and tasks (`.granit/agent/tasks/<name>.md`), the `*_skill` / `*_task` commands in `src-tauri/src/commands/`, with Tera variable help. Missing in the GPUI app (zem.5c inventory); `/task` in chat already lists tasks.

## Acceptance Criteria

- [ ] Skills and tasks can be listed, created, renamed, deleted and edited in the editor (a new document kind next to note and template)
- [ ] Help text for the Tera variables available to tasks

## Testing

- `gpui::test`s for create/edit/rename/delete