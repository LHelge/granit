---
id: see
title: Log warnings for secrets.env parse failures
status: open
priority: P3
created: 2026-03-31T16:33:41.283035363Z
updated: 2026-03-31T16:33:41.283035363Z
tags:
- backend
parent: 4cm
---

## Summary
In config/mod.rs, `load_env_file` catches all dotenvy parsing errors silently via `if let Ok(iter)` and `.flatten()`. Users won't know why their API key isn't loading if their secrets.env has a syntax error.

## Acceptance Criteria
- [ ] Log a warning (via `eprintln!` or `log::warn!`) when secrets.env exists but fails to parse
- [ ] Individual line parse errors should also be logged (not flattened away)

## Implementation Notes
- Files: `src-tauri/src/config/mod.rs` (load_env_file, around line 277)
- Keep the app running even on parse failure — just log, don't fail open