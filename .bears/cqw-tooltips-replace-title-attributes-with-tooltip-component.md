---
id: cqw
title: 'Tooltips: Replace title attributes with tooltip component'
status: done
priority: P2
created: 2026-04-04T21:14:52.314397247Z
updated: 2026-04-04T22:03:53.259198863Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace native `title` attributes with DaisyUI `tooltip` component for styled, consistent tooltips across toolbar buttons and icon buttons.

## Acceptance Criteria

- [ ] Toolbar buttons (toggle sidebar, toggle agent, daily note) use `tooltip tooltip-bottom` with `data-tip`
- [ ] Editor action buttons (edit, save, cancel) use `tooltip`
- [ ] Agent panel clear chat button uses `tooltip`
- [ ] Settings gear button uses `tooltip`
- [ ] Provider remove button uses `tooltip`

## Implementation Notes

- DaisyUI tooltip wraps the element: `<div class="tooltip" data-tip="text"><button>...</button></div>`
- Need to ensure tooltips don't conflict with absolute positioning in floating action buttons

## Files to Modify

- `src/app/mod.rs` — titlebar buttons
- `src/app/components/agent_panel.rs` — clear chat button
- `src/app/components/cave_selector.rs` — settings gear
- `src/app/components/editor/mod.rs` — floating action buttons
- `src/app/components/settings/agent.rs` — remove provider, show/hide key
