---
id: kgf
title: 'Loading: Use loading component for streaming and model loading'
status: done
priority: P2
created: 2026-04-04T21:14:57.721206885Z
updated: 2026-04-04T21:54:36.701977099Z
tags:
- frontend
depends_on:
- 9ma
parent: 9fd
---

## Summary

Replace custom loading indicators (animated pulse cursors, "Loading..." text) with DaisyUI `loading` component for consistent spinner/dots styling.

## Acceptance Criteria

- [ ] Agent streaming cursor uses `loading loading-dots loading-sm` instead of custom pulse animation
- [ ] Model selector loading state uses `loading loading-spinner loading-xs`
- [ ] Settings save spinner uses `loading loading-spinner loading-xs`

## Files to Modify

- `src/app/components/agent_panel.rs` — streaming cursor indicator
- `src/app/components/model_selector.rs` — "Loading..." text
