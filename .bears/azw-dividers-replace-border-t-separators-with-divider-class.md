---
id: azw
title: 'Dividers: Replace border-t separators with divider class'
status: done
priority: P2
created: 2026-04-04T21:15:02.703788175Z
updated: 2026-04-04T21:55:37.633655368Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace manual `border-t` / `border-b` separator divs with DaisyUI `divider` class where appropriate.

## Acceptance Criteria

- [ ] Cave selector dropdown separator uses `divider` or `<li>` within menu
- [ ] Settings agent section `<hr>` uses `divider` class
- [ ] Only replace separators within content areas — structural borders (panel edges, header borders) should remain as-is

## Files to Modify

- `src/app/components/cave_selector.rs` — dropdown divider
- `src/app/components/settings/agent.rs` — hr between font settings and providers
