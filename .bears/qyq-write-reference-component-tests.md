---
id: qyq
title: Write reference component tests
status: cancelled
priority: P2
created: 2026-03-31T16:40:34.556669077Z
updated: 2026-03-31T20:08:41.742850216Z
tags:
- frontend
depends_on:
- kk6
parent: 82y
---

**Deferred**: Depends on `kk6` (IPC trait/mock) which was cancelled. Components that call `ipc::*` functions would panic in tests without a mock Tauri IPC layer. Can be revisited when IPC mockability is added.