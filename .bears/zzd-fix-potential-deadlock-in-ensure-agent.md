---
id: zzd
title: Fix potential deadlock in ensure_agent()
status: open
priority: P1
created: 2026-04-04T21:40:31.625350180Z
updated: 2026-04-04T21:40:31.625350180Z
tags:
- errors
- backend
depends_on:
- eup
parent: drc
---

## Summary
`ensure_agent()` holds the agent lock while acquiring the config lock. If any other code path acquires config then agent, we get a deadlock.

## What to do
- Reorder lock acquisition: read config first, drop the guard, then lock agent
- Pattern: `let config_data = { let c = self.config.lock()?; (c.agent.clone(), ...) }; let mut agent = self.lock_agent()?;`

## Files
- `src-tauri/src/lib.rs` — `ensure_agent()` method