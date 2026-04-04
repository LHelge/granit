---
id: 3kk
title: Extract cave child-path update helper
status: done
priority: P2
created: 2026-04-04T21:41:02.995890694Z
updated: 2026-04-04T22:45:23.744281507Z
tags:
- duplication
- backend
- cave
parent: drc
---

## Summary
`move_folder()` and `rename_folder()` in cave/mod.rs have identical 8-line blocks that update child note paths after a directory rename/move.

## What to do
- Extract `fn update_child_paths(&mut self, old_prefix: &Path, new_prefix: &Path)`
- Call from both `move_folder()` and `rename_folder()`

## Files
- `src-tauri/src/cave/mod.rs`