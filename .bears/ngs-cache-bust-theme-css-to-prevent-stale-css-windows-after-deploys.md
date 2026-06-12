---
id: ngs
title: Cache-bust theme.css to prevent stale-CSS windows after deploys
status: in_progress
priority: P2
created: "2026-06-12T15:42:03.207410963Z"
updated: "2026-06-12T15:42:15.611410758Z"
tags:
  - docs
  - theme
  - bug
---

## Summary

Chrome on iPhone showed both toggle icons and was stuck in dark mode: the HTML had updated after the toggle deploy but the cached pre-toggle `theme.css` (Pages max-age=600) had no `.theme-toggle` or `[data-theme]` rules. Version the stylesheet URL so HTML and CSS can't go out of sync.

## Acceptance Criteria

- [ ] Stylesheet link carries a version query bumped whenever theme.css changes
- [ ] theme.toml version bumped to match
- [ ] Comment in base.html explaining the bump-on-change rule