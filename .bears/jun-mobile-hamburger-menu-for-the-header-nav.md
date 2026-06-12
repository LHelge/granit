---
id: jun
title: Mobile hamburger menu for the header nav
status: in_progress
priority: P2
created: "2026-06-12T15:48:23.540606229Z"
updated: "2026-06-12T15:48:31.649002025Z"
tags:
  - docs
  - theme
  - css
---

## Summary

The header nav text links (Wiki/Download/About) overlap the brand on narrow phones. Collapse them into a CSS-only `<details>` hamburger dropdown below 40rem; theme toggle and GitHub icon stay inline.

## Acceptance Criteria

- [ ] Desktop unchanged; below 40rem the text links hide and a hamburger appears
- [ ] Dropdown panel on the Mocha/Latte surface with proper tap targets, closes on navigation
- [ ] No JS; works with the theme toggle in both schemes
- [ ] Stylesheet ?v= and theme.toml bumped per the cache-bust convention