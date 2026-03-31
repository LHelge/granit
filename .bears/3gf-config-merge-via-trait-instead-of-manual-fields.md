---
id: 3gf
title: Config merge via trait instead of manual fields
status: open
priority: P2
created: 2026-03-31T16:32:52.076973346Z
updated: 2026-03-31T16:32:52.076973346Z
tags:
- backend
- refactor
parent: 4cm
---

## Summary
The `merge()` function in config/mod.rs manually chains `.and_then()` for every field across font configs and agent config (~26 lines of nested assignments). Forgetting a new field is a silent bug.

## Acceptance Criteria
- [ ] Create a `Merge` trait: `fn merge(self, overlay: Option<Self>) -> Self`
- [ ] Implement for `AgentConfig`, `FontConfig`, and top-level config
- [ ] Replace manual field-by-field merge with trait calls
- [ ] Existing tests still pass

## Implementation Notes
- Files: `src-tauri/src/config/mod.rs`, `granit-types/src/config.rs`
- The trait could live in `granit-types` since both layers use it
- Consider `impl Merge for FontConfig` where each field uses `overlay.field.unwrap_or(self.field)`
- RawConfig fields are `Option<T>` — the trait merges `T` with `Option<T>` overlay

## Testing
- Existing config merge tests should still pass unchanged