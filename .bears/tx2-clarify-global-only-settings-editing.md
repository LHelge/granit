---
id: tx2
title: Clarify global-only settings editing
status: open
priority: P3
created: 2026-03-27T12:31:56.443401Z
updated: 2026-03-27T12:31:56.443401Z
parent: wem
---

## Summary

Align the current settings UX, command surface, and documentation around global-only config editing for now. The backend can load cave overrides, but the active UI should not imply scope-aware settings behavior until per-cave editing is intentionally designed.

## Acceptance Criteria

- [ ] The current settings UI clearly behaves as a global settings editor.
- [ ] Documentation does not imply that users can currently edit cave-local overrides through the UI.
- [ ] Any misleading naming or command semantics around config scope are clarified.
- [ ] The deferred nature of per-cave settings is clear in the codebase and docs.

## Implementation Notes

- Review src/app/components/settings_modal.rs, src-tauri/src/lib.rs, src-tauri/src/config/mod.rs, and README.md.
- Keep cave override loading support if it is still useful for future work, but do not present it as an active UI feature.
- Coordinate with the docs-alignment task.

## Testing

- Manual verification is sufficient: confirm the UI wording and docs no longer imply per-cave editing support.
