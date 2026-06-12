---
id: bww
title: Document or replace pending() async pattern
status: done
priority: P3
created: 2026-03-31T16:33:41.276152370Z
updated: 2026-03-31T20:22:21.141957080Z
tags:
- frontend
- refactor
parent: 4cm
---

## Summary
In agent_panel.rs, `std::future::pending::<()>().await` is used inside an Effect to keep event listener handles alive. This is clever but undocumented and fragile — could break if Leptos Effect cleanup semantics change.

## Acceptance Criteria
- [ ] Add a SAFETY comment explaining why this pattern works and its assumptions
- [ ] Investigate whether there's a more idiomatic Leptos approach (e.g. `on_cleanup`)
- [ ] If a safer alternative exists, migrate to it

## Implementation Notes
- Files: `src/app/components/agent_panel.rs` (lines ~16-54)
- The pattern relies on Effect keeping the spawned future alive, and dropping it on cleanup
- Check Leptos docs for `on_cleanup` or `StoredValue` as alternatives