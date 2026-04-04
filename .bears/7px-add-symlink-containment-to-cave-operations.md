---
id: 7px
title: Add symlink containment to cave operations
status: open
priority: P3
created: 2026-04-04T21:41:36.956878091Z
updated: 2026-04-04T21:41:36.956878091Z
tags:
- security
- backend
- cave
parent: drc
---

## Summary
Cave path validation rejects `..` but doesn't resolve symlinks. A symlink inside the cave could allow operations on files outside the cave root.

## What to do
- After `self.path.join(user_path)`, call `canonicalize()` on the result
- Verify the canonical path starts with `canonicalize(&self.path)`
- Apply to all path operations that accept user input (create, move, rename)

## Risk
Low in single-user desktop app, but worth fixing for correctness.

## Files
- `src-tauri/src/cave/mod.rs` — path validation functions
- `src-tauri/src/cave/note.rs` — validate_folder_path()