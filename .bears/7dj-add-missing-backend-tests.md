---
id: 7dj
title: Add missing backend tests
status: done
priority: P2
created: 2026-04-04T21:41:45.785242051Z
updated: 2026-04-04T22:59:27.465165380Z
tags:
- testing
- backend
depends_on:
- mef
parent: drc
---

## Summary
Fill testing gaps identified during review.

## Tests to add
1. Config load/save round-trip with the simplified single-file config
2. Config deserialization with missing fields (serde defaults kick in)
3. `rebuild_with_frontmatter()` strips frontmatter from new_body correctly
4. Slug resolution case-sensitivity edge cases on case-sensitive filesystems
5. `to_ipc()` path conversion (non-UTF8, relative, empty paths)

## Files
- `src-tauri/src/config/mod.rs` — config tests
- `src-tauri/src/markdown/mod.rs` — frontmatter tests
- `src-tauri/src/cave/mod.rs` — slug tests