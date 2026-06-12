---
id: swf
title: 'Menu: Use menu component for context menus'
status: done
priority: P1
created: 2026-04-04T21:14:41.360895683Z
updated: 2026-04-04T21:48:57.000087310Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace hand-rolled context menu styling with DaisyUI `menu` component. The tree view context menu currently uses a custom div with manually styled buttons — DaisyUI menu provides consistent item spacing and hover styling.

## Acceptance Criteria

- [ ] Context menu container uses `menu bg-base-300 rounded-box shadow-lg` classes
- [ ] Menu items use `<li><button>...</button></li>` pattern with menu component
- [ ] Danger items (delete) use appropriate error color styling within menu
- [ ] Cave selector dropdown items also converted to menu

## Files to Modify

- `src/app/components/tree_view/context_menu.rs` — note/folder/root menus
- `src/app/components/cave_selector.rs` — recent caves dropdown menu
