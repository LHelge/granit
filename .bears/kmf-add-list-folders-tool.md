---
id: kmf
title: Add list_folders tool
status: done
priority: P1
created: 2026-04-05T11:04:10.111767619Z
updated: 2026-04-05T11:14:13.644554161Z
tags:
- agent
depends_on:
- fmh
parent: fm9
---

## Summary
Add `list_folders` tool to the navigation module. The cave API already has `cave.list_folders()`.

## Implementation
- Add `ListFoldersTool` to `navigation.rs`
- No args needed (empty struct like ListNotesTool)
- Returns list of folder paths (relative to cave root)
- Register in `build_toolset`, add to `TOOL_CATALOGUE`

## Acceptance Criteria
- [ ] Agent can list all folders in the cave
- [ ] Tool registered and appears in settings UI