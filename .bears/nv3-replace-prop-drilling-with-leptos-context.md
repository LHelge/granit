---
id: nv3
title: Replace prop drilling with Leptos context
status: open
priority: P2
created: 2026-03-31T16:32:52.085499385Z
updated: 2026-03-31T16:32:52.085499385Z
tags:
- frontend
- refactor
parent: 4cm
---

## Summary
Sidebar accepts 6 separate signal parameters. Editor, CaveSelector similarly pass many signals down. Signals like `config`, `error_msg`, `notes_error` are used by many components and should be provided via Leptos context.

## Acceptance Criteria
- [ ] Move shared signals (`config`, `error_msg`, `notes_error`, `notes`, `active_note`) to `provide_context` in App
- [ ] Components use `use_context` / `expect_context` instead of props
- [ ] Sidebar, CaveSelector, Editor prop lists are reduced

## Implementation Notes
- Files: `src/app/mod.rs`, `src/app/components/sidebar.rs`, `src/app/components/cave_selector.rs`, `src/app/components/editor/mod.rs`
- Consider a context struct: `struct AppCtx { config: RwSignal<AppConfig>, error: RwSignal<Option<String>>, ... }`
- The `OpenInEdit` context already uses this pattern — follow it

## Testing
- App compiles and behaves identically