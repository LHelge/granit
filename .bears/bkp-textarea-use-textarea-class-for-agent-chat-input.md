---
id: bkp
title: 'Textarea: Use textarea class for agent chat input'
status: open
priority: P1
created: 2026-04-04T21:14:03.537750025Z
updated: 2026-04-04T21:14:03.537750025Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace the manual textarea styling in the agent chat input with DaisyUI `textarea textarea-bordered` class.

## Acceptance Criteria

- [ ] Agent chat textarea uses `textarea textarea-bordered w-full` instead of hand-rolled classes
- [ ] Focus ring handled by DaisyUI
- [ ] Resize behavior preserved (`resize-none`)

## Files to Modify

- `src/app/components/agent_panel.rs` — message textarea
