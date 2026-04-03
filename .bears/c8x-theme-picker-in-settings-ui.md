---
id: c8x
title: Theme picker in settings UI
status: open
priority: P2
created: 2026-04-02T23:14:24.872128611Z
updated: 2026-04-02T23:14:24.872128611Z
tags:
- frontend
depends_on:
- ffj
- f6k
parent: njk
---

## Summary
Add a theme picker to the settings UI, allowing the user to browse and switch between available themes.

## Acceptance Criteria
- [ ] Theme section in settings modal/panel
- [ ] Dropdown or grid showing all available themes (name + light/dark indicator)
- [ ] Selecting a theme calls `set_active_theme` and applies it immediately
- [ ] Current active theme is highlighted/selected
- [ ] Optional: small color preview swatches per theme

## Implementation Notes
- Add to `src/app/components/settings/mod.rs` (or new `theme_picker.rs`)
- Fetch theme list via `invoke("list_themes")`
- On selection, call `invoke("set_active_theme", { id })` then re-inject CSS vars
- Follows existing settings UI patterns (dropdowns, labels)