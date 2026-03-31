---
id: wqu
title: Enforce slug uniqueness invariant at cave open
status: done
priority: P1
created: 2026-03-31T16:32:23.229756707Z
updated: 2026-03-31T16:50:42.133846740Z
tags:
- backend
- bug
parent: 4cm
---

## Summary
`scan_recursive` logs duplicate slugs but silently skips the second copy. Downstream code assumes slug→path is 1:1, which can break if duplicates exist from external edits.

## Acceptance Criteria
- [ ] At cave open, if duplicate slugs are detected, return an error (or surface a warning to the user)
- [ ] Consider a newtype `CaveSlug(String)` to encode the uniqueness invariant at the type level
- [ ] Tests for the duplicate detection path

## Implementation Notes
- Files: `src-tauri/src/cave/mod.rs` (scan_recursive, lines ~46-68)
- Option A: `Cave::open()` returns `Err(CaveError::DuplicateSlug { slug, paths })` 
- Option B: Collect duplicates and surface as warnings, let user resolve
- Prefer Option A for correctness

## Testing
- Create temp dir with `folder1/test.md` and `folder2/test.md`, verify `Cave::open()` fails with meaningful error