---
id: wpn
title: Unify error signals into single channel
status: done
priority: P2
created: 2026-03-31T16:33:19.406421967Z
updated: 2026-03-31T20:08:49.575917776Z
tags:
- frontend
- refactor
depends_on:
- nv3
parent: 4cm
---

## Summary
App has two separate error signals (`error_msg` and `notes_error`) with different display patterns. Should unify into a single error channel.

## Acceptance Criteria
- [ ] Single error signal (or signal holding a Vec of errors with source/severity)
- [ ] Error banner shows all active errors, dismissible individually
- [ ] Components push errors to one channel instead of separate signals

## Implementation Notes
- Files: `src/app/mod.rs`, `src/app/components/sidebar.rs`, `src/app/components/cave_selector.rs`
- Consider: `struct AppError { source: &'static str, message: String }` with `RwSignal<Vec<AppError>>`
- This pairs well with the prop drilling → context refactor (task nv3)

## Testing
- App compiles; errors display correctly