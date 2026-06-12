---
id: f6k
title: Migrate all frontend components to theme tokens
status: done
priority: P1
created: 2026-04-02T23:13:25.315241784Z
updated: 2026-04-03T23:51:13.938262387Z
tags:
- frontend
depends_on:
- 7mj
parent: njk
---

## Summary
Replace all hardcoded `stone-*` and `red-*` Tailwind color classes across every frontend component with the new semantic theme tokens.

## Acceptance Criteria
- [ ] No remaining hardcoded `stone-*` or `red-*` color classes in any `.rs` frontend file
- [ ] All components use theme-aware classes (e.g. `bg-base`, `text-text`, `bg-surface0`, `border-overlay0`)
- [ ] Visual appearance matches current dark theme when Mocha is active
- [ ] Light theme (Latte) renders correctly with proper contrast

## Files to Update
- `src/app/mod.rs` — app container, header, error toasts
- `src/app/components/sidebar.rs`
- `src/app/components/agent_panel.rs`
- `src/app/components/editor/mod.rs`
- `src/app/components/editor/reader.rs`
- `src/app/components/editor/writer.rs`
- `src/app/components/settings/mod.rs`
- `src/app/components/settings/agent.rs`
- `src/app/components/settings/font_picker.rs`
- `src/app/components/tree_view/mod.rs`
- `src/app/components/tree_view/note_node.rs`
- `src/app/components/tree_view/context_menu.rs`
- `src/app/components/provider_selector.rs`
- `src/app/components/model_selector.rs`
- `src/app/components/cave_selector.rs`

## Mapping Guide (current → new)
- `bg-stone-900` → `bg-base`
- `bg-stone-850` → `bg-mantle`
- `bg-stone-800` → `bg-surface0`
- `bg-stone-700` → `bg-surface1`
- `text-stone-200` → `text-text`
- `text-stone-400` → `text-subtext0`
- `text-stone-500` → `text-overlay1`
- `border-stone-600` → `border-overlay0`
- `border-stone-700` → `border-surface2`
- `red-*` error states → `red` theme color equivalents