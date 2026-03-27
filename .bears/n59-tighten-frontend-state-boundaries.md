---
id: n59
title: Tighten frontend state boundaries
status: done
priority: P3
created: 2026-03-27T12:36:52.467392Z
updated: 2026-03-27T14:37:17.752087Z
depends_on:
- c2z
- 3s4
parent: wem
---

## Summary

Refine frontend state ownership and component boundaries so the UI is easier to evolve without unnecessary prop drilling, stale local state, or mixed responsibilities. This is a bounded maintainability refactor, not a large architecture rewrite.

## Acceptance Criteria

- [ ] Obvious prop-drilling hotspots are reduced where it meaningfully improves clarity.
- [ ] Component responsibilities are clearer, especially around sidebar, cave selection, and editor-adjacent state.
- [ ] Avoidable stale local state is reduced.
- [ ] The refactor stays proportional and does not introduce unnecessary abstractions.

## Implementation Notes

- Review src/app/mod.rs, src/app/components/sidebar.rs, src/app/components/cave_selector.rs, and src/app/components/settings_modal.rs.
- Prefer small structural improvements over a broad context-heavy redesign.
- Coordinate with IPC error handling and current-cave-state work, since those changes may naturally simplify some state flow.

## Testing

- Manual verification of the main UI flows is sufficient unless logic is extracted into testable units.
