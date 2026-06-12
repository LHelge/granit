---
id: 93h
title: Add open_daily_note tool
status: done
priority: P2
created: 2026-04-05T11:04:41.342981914Z
updated: 2026-04-05T11:57:14.268286673Z
tags:
- agent
depends_on:
- fmh
parent: fm9
---

## Summary
Add `open_daily_note` tool. Cave API already has `cave.open_daily_note(folder)`.

## Implementation (new file `daily.rs`)

### open_daily_note
- `OpenDailyNoteArgs { folder: Option<String> }`
- Calls `cave.open_daily_note(folder.as_deref().unwrap_or("Daily"))`
- Returns slug, relative path, and content of today's note

## Registration
- Add to `build_toolset`, `TOOL_CATALOGUE`
- No special param extraction needed in `build_tool_call_info`

## Acceptance Criteria
- [ ] Agent can create/open today's daily note
- [ ] Tool registered and visible in settings