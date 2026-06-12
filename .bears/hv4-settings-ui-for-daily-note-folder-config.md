---
id: hv4
title: Settings UI for daily note folder config
status: done
priority: P2
created: 2026-04-03T23:11:37.094830783Z
updated: 2026-04-03T23:21:13.884651636Z
tags:
- frontend
- settings
- ui
depends_on:
- kf9
parent: hh6
---

## Summary
Add a daily_note_folder input in the settings modal so the user can customize which folder daily notes go into.

## Acceptance Criteria
- [ ] Settings modal shows a text input for "Daily note folder" (default: "Daily")
- [ ] Located in a general/notes section (or a new "Notes" section if appropriate)
- [ ] On save, the value persists to the config and is reflected in the topbar button behavior
- [ ] Empty string or whitespace resets to default "Daily"

## Implementation Notes
- Location: `src/app/components/settings/mod.rs` or a sub-component
- Add a signal for the field, populate from `ctx.config.get().daily_note_folder`
- On save, include the field in the config payload sent to `save_config()`
- Follow existing settings field patterns (font inputs, etc.)

## Edge Cases
- User enters folder with slashes (e.g. "Notes/Daily") — should be accepted
- User clears the field → use default "Daily"