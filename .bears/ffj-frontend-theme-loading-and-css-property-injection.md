---
id: ffj
title: Frontend theme loading and CSS property injection
status: done
priority: P1
created: 2026-04-02T23:14:06.550649695Z
updated: 2026-04-03T23:51:16.313360032Z
tags:
- frontend
depends_on:
- t2e
- 7mj
parent: njk
---

## Summary
Add frontend IPC calls to fetch the active theme on startup and apply it by setting CSS custom properties on `:root`. Ensure theme changes apply immediately without reload.

## Acceptance Criteria
- [ ] On app startup, fetch active theme via `invoke("get_active_theme")`
- [ ] Apply theme colors as CSS custom properties on `document.documentElement`
- [ ] Theme signal/state available to components that need to react
- [ ] When theme is changed via settings, re-apply CSS properties immediately
- [ ] No flash of unstyled/wrong-theme content on startup

## Implementation Notes
- Use `wasm_bindgen` / `web_sys` to set CSS properties on `:root`
- Call in `app.rs` initialization (or a dedicated theme provider component)
- IPC wrapper in `src/app/ipc.rs`