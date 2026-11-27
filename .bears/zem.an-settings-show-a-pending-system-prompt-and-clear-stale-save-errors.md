---
id: zem.an
title: "Settings: show a pending system prompt and clear stale save errors"
status: open
priority: P3
created: "2026-11-27T06:10:39.694883485Z"
updated: "2026-11-27T06:10:39.694883485Z"
tags:
  - gpui
  - port
  - frontend
  - settings
depends_on:
  - zem.dz
---

## Summary

Left open after the second review round of zem.dz:

- `granit-app/src/settings/mod.rs:95`: reopened settings fill the system-prompt textarea from disk (`config_ops::read_system_prompt`), ignoring a prompt still pending in `GlobalPendingPrompt` after a failed write. Closing without touching it then writes the pending text the user cannot see; editing the shown text silently replaces the pending one. Start the textarea from the pending prompt when it belongs to the open cave (e.g. `config_ops::pending_system_prompt(cx)`).
- After a failed save, a later successful deferred commit (`commit_edits`) never calls `report(Ok)`, so the error above the pages stays until some other write succeeds. Clear it on a successful commit (e.g. on `ConfigEvent`).

## Acceptance Criteria

- [ ] Reopened settings show the pending prompt, and what they show is what gets saved
- [ ] A successful deferred save clears a stale error when nothing else is unsaved

## Testing

- Extend `a_system_prompt_that_cannot_be_written_outlives_the_settings` to reopen the settings and check the textarea