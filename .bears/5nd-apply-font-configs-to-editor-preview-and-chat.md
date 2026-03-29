---
id: 5nd
title: Apply font configs to editor, preview, and chat
status: open
priority: P1
created: 2026-03-29T21:02:55.549080253Z
updated: 2026-03-29T21:02:55.549080253Z
tags:
- frontend
depends_on:
- e2f
parent: bp2
---

## Summary

Apply the configured font family and font size to the editor textarea, markdown preview pane, and agent chat panel using reactive inline styles.

## Acceptance Criteria

- [ ] Editor textarea uses markdown font config
- [ ] Preview pane uses reading font config
- [ ] Agent chat uses agent font config
- [ ] Changes take effect immediately when config signal updates

## Implementation Notes

- Files: `src/app/components/editor.rs`, `src/app/components/agent_panel.rs`
- Apply via `style:font-family` and `style:font-size` in Leptos `view!` macros