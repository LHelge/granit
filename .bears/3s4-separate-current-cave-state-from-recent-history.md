---
id: 3s4
title: Separate current cave state from recent history
status: done
priority: P2
created: 2026-03-27T12:25:48.767022Z
updated: 2026-03-27T13:18:19.970923Z
parent: wem
---

## Summary

Separate the app’s runtime notion of the currently open cave from the persisted list of recent caves. The current implementation uses `recent_caves.first()` as if it were the active cave, which conflates history with live state and makes error handling and future features more brittle.

## Acceptance Criteria

- [ ] The backend tracks the currently open cave explicitly.
- [ ] The frontend can distinguish the active cave from the recent-caves list.
- [ ] Startup and cave-opening flows do not rely on `recent_caves.first()` as the live source of truth.
- [ ] Recent caves remain a persistence/history feature rather than a runtime state proxy.
- [ ] Tests or focused verification cover the updated behavior.

## Implementation Notes

- Review src-tauri/src/lib.rs, src-tauri/src/config/mod.rs, src/app/mod.rs, src/app/components/sidebar.rs, and src/app/components/cave_selector.rs.
- Keep the recent-cave list as persisted config, but return active-cave information separately if needed.
- Coordinate with IPC error handling work so cave-open failures do not leave the UI in an ambiguous state.

## Edge Cases

- Startup with a recent cave path that no longer exists.
- Cave-open failure after a user picks a folder.
- Empty recent-caves list with no active cave.

## Testing

- Add backend or integration coverage for cave-open and startup-state behavior where practical.
- cargo fmt && cargo clippy && cargo test must pass.
