---
id: zem.dz
title: "Settings: handle a failed deferred config save"
status: open
priority: P2
created: "2026-11-27T01:51:05.838414673Z"
updated: "2026-11-27T01:51:05.838414673Z"
tags:
  - gpui
  - port
  - frontend
  - settings
depends_on:
  - zem.2u
---

## Summary

Review of zem.2u left these open on the deferred settings save (`granit-app/src/config_ops.rs`, `edit_config`/`flush_config`):

- `granit-app/src/workspace.rs:264-267`: when `open_cave` flushes the settings and the save fails (e.g. read-only `config.yml`), the switch carries on; `Opened` closes the settings, the error vanishes with them and the edit is lost silently. Refuse the switch and show the error in `open_error`, as unsaved note edits already do.
- `config_ops.rs:178-223`: after a failed save the typed value stays in memory but the pending edit is discarded, so the next save compares against a baseline that already contains it; an agent-field edit then never resets the agent. Restore the baseline or keep the pending edit on failure.
- Number fields commit only after the 500 ms pause (not on Enter/blur like `text_field`), so a chat message sent within that window is cleared by the later reset.
- The `update_config` doc comment says nothing changes when saving fails; no longer true for pending edits.

## Acceptance Criteria

- [ ] A failed settings save before a cave switch refuses the switch and shows the error
- [ ] After a failed save, a later successful save still resets the agent for agent-field edits
- [ ] Number fields commit on Enter and blur
- [ ] Doc comment corrected

## Testing

- `gpui::test`s with a read-only `config.yml`