---
id: jmn
title: 'Toasts: Use toast and alert components for error notifications'
status: open
priority: P1
created: 2026-04-04T21:14:13.468424232Z
updated: 2026-04-04T21:14:13.468424232Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace the hand-rolled toast notification container with DaisyUI `toast` positioning and `alert` component for individual notifications.

## Acceptance Criteria

- [ ] Toast container uses `toast toast-end toast-bottom` for positioning
- [ ] Individual error notifications use `alert alert-error` with compact sizing
- [ ] Editor error banner uses `alert alert-error` component
- [ ] Agent stream error uses `alert alert-error`
- [ ] Dismiss button preserved

## Files to Modify

- `src/app/mod.rs` — toast notification container at bottom-right
- `src/app/components/editor/mod.rs` — error banner above content
- `src/app/components/agent_panel.rs` — stream error display
