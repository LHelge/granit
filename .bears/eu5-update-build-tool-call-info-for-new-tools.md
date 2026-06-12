---
id: eu5
title: Update build_tool_call_info for new tools
status: done
priority: P2
created: 2026-04-05T11:04:48.269048446Z
updated: 2026-04-05T11:57:26.284108165Z
tags:
- agent
depends_on:
- kmf
- zha
- jwr
- ee7
- 93h
parent: fm9
---

## Summary
Update `build_tool_call_info` in `agent/mod.rs` to extract meaningful params from all new tools for the chat UI.

## Param mapping for new tools
- `move_note`, `rename_note` → extract `slug`
- `create_folder`, `rename_folder`, `delete_folder` → extract `path`
- `move_folder` → extract `source`
- `search_content` → extract `query`
- `open_daily_note` → extract `folder`
- `list_folders` → no param

## Acceptance Criteria
- [ ] All new tools show meaningful params in chat UI tool call indicators