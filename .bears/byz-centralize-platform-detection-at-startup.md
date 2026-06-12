---
id: byz
title: Centralize platform detection at startup
status: done
priority: P3
created: 2026-03-31T16:33:41.284957324Z
updated: 2026-03-31T20:08:49.586108941Z
tags:
- frontend
- refactor
depends_on:
- nv3
parent: 4cm
---

## Summary
Platform detection via `js_sys::eval("navigator.platform")` is used in two places (editor/mod.rs for keyboard shortcuts, app/mod.rs for titlebar margin). Should detect once at startup and cache.

## Acceptance Criteria
- [ ] Detect platform once in App component, store in Leptos context
- [ ] Replace both `js_sys::eval` calls with context lookup
- [ ] Use `web_sys::window().navigator().platform()` instead of eval for safety

## Implementation Notes
- Files: `src/app/mod.rs`, `src/app/components/editor/mod.rs`
- Create `struct Platform { is_mac: bool }` context signal
- This pairs well with the prop drilling → context refactor (task nv3)