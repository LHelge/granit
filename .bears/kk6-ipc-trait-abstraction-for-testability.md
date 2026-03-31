---
id: kk6
title: IPC trait abstraction for testability
status: cancelled
priority: P1
created: 2026-03-31T16:40:34.554823843Z
updated: 2026-03-31T20:00:20.013211635Z
tags:
- frontend
- refactor
depends_on:
- tfv
parent: 82y
---

**Deferred**: The three generic helpers (`invoke_cmd`, `invoke_no_args`, `invoke_unit`) already centralize all IPC logic. A full async trait is complex in WASM (no `Send` bounds) and would only benefit component tests, which don't exist yet. Can be revisited when component testing is added.