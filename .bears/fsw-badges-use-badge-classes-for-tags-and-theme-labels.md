---
id: fsw
title: 'Badges: Use badge classes for tags and theme labels'
status: open
priority: P1
created: 2026-04-04T21:14:08.538028416Z
updated: 2026-04-04T21:14:08.538028416Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace hand-rolled pill/badge styling with DaisyUI `badge` component classes for frontmatter tags and theme dark/light labels.

## Acceptance Criteria

- [ ] Frontmatter tag pills use `badge badge-sm` with close button
- [ ] Theme dark/light labels in theme picker use `badge badge-xs`
- [ ] Active theme indicator uses `badge badge-primary badge-xs`
- [ ] Tool call indicators in agent panel use `badge badge-ghost badge-sm`

## Files to Modify

- `src/app/components/editor/frontmatter.rs` — tag pills
- `src/app/components/settings/theme.rs` — dark/light labels, active indicator
- `src/app/components/agent_panel.rs` — tool call display
