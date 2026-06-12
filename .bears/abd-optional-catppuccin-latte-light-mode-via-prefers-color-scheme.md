---
id: abd
title: "Optional: Catppuccin Latte light mode via prefers-color-scheme"
status: open
priority: P3
created: "2026-06-12T11:39:10.613089914Z"
updated: "2026-06-12T11:39:10.613089914Z"
tags:
  - docs
  - polish
  - css
depends_on:
  - u8w
parent: hb5
---

## Summary

Pure-polish follow-up: a Catppuccin Latte token block behind `@media (prefers-color-scheme: light)`. No JS toggle — system preference only.

## Acceptance Criteria

- [ ] Latte palette overrides the `:root` custom properties in a `prefers-color-scheme: light` block (Latte values from the app's `styles.css` catppuccin-latte theme)
- [ ] `color-scheme: light dark`; `theme-color` meta handled for both schemes
- [ ] All page types legible in light mode incl. `hl-*` code colors (Latte syntax mapping) and alert cards

## Implementation Notes

- Skip or defer freely — Mocha-only is the accepted v1. Only do this after launch.