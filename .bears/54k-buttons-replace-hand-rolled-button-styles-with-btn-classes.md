---
id: 54k
title: 'Buttons: Replace hand-rolled button styles with btn classes'
status: done
priority: P1
created: 2026-04-04T21:13:53.669925203Z
updated: 2026-04-04T21:22:28.877671497Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace verbose hand-rolled Tailwind button classes with DaisyUI `btn` variants across all frontend components. This is the highest-impact change since buttons appear everywhere.

## Acceptance Criteria

- [ ] All `<button>` elements use `btn` + appropriate variant classes
- [ ] Toolbar icon buttons use `btn btn-ghost btn-xs btn-square` or `btn btn-ghost btn-sm btn-circle`
- [ ] Action buttons (Save, Cancel, Send) use `btn btn-sm` with color variants
- [ ] Destructive buttons use `btn-error` or `btn-ghost` with error text color
- [ ] Disabled state uses `btn-disabled` class or native `disabled` attribute (both work with DaisyUI)
- [ ] No visual regressions

## Files to Modify

- `src/app/mod.rs` — titlebar toggle buttons, daily note button
- `src/app/components/agent_panel.rs` — clear chat, send button
- `src/app/components/cave_selector.rs` — cave dropdown trigger, settings gear, open folder
- `src/app/components/provider_selector.rs` — dropdown trigger, dropdown items
- `src/app/components/model_selector.rs` — dropdown trigger, dropdown items
- `src/app/components/settings/mod.rs` — close, cancel, save buttons
- `src/app/components/settings/agent.rs` — add provider, remove provider, type selector, show/hide key
- `src/app/components/editor/mod.rs` — edit/save/cancel floating action buttons
- `src/app/components/editor/frontmatter.rs` — tag remove button
- `src/app/components/editor/icon_picker.rs` — icon buttons
- `src/app/components/tree_view/context_menu.rs` — menu item buttons
