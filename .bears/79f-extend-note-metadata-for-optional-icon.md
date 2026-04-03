---
id: 79f
title: Extend note metadata for optional icon
status: open
priority: P1
created: 2026-04-03T14:10:13.567383222Z
updated: 2026-04-03T14:10:13.567383222Z
tags:
- backend
- types
- notes
parent: gcy
---

## Summary

Add optional icon support to the shared note metadata contract and backend frontmatter parsing so note icons can be stored as normalized strings in frontmatter and surfaced through Rust types without breaking existing notes.

## Acceptance Criteria
- [ ] `Frontmatter` supports an optional `icon` field.
- [ ] `NoteMeta` supports an optional `icon` field for list/tree consumption.
- [ ] Backend exposes a frontmatter parsing path that can read `icon` without rendering markdown.
- [ ] Existing notes without `icon` remain fully compatible.

## Implementation Notes
- Update shared note types in `granit-types/src/note.rs`.
- Reuse `src-tauri/src/markdown/mod.rs` frontmatter parsing instead of duplicating YAML handling.
- Keep unknown or missing values as `None` or pass them through safely.

## Testing
- Add or update tests for icon frontmatter parsing.
- `cargo fmt && cargo clippy && cargo test` should pass.
