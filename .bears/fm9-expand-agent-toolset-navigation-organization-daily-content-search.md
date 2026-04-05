---
id: fm9
title: Expand agent toolset — navigation, organization, daily, content search
type: epic
status: done
priority: P1
created: 2026-04-05T11:03:47.408788363Z
updated: 2026-04-05T11:57:26.284302043Z
tags:
- agent
- backend
---

## Scope

Expand the agent's cave tools from 7 note tools to a complete set covering navigation, reading, writing, organization, daily workflows, and content search. Also reorganize tool source files from one-per-tool to grouped by category.

## New Tool Categories & File Structure

```
src-tauri/src/agent/tools/
  mod.rs          — SharedCave, ToolError, helpers, build_toolset, TOOL_CATALOGUE
  navigation.rs   — list_notes, list_folders, search_notes, search_content
  reading.rs      — read_note
  writing.rs      — create_note, update_note, edit_note
  organization.rs — move_note, rename_note, create_folder, rename_folder, move_folder, delete_note, delete_folder
  daily.rs        — open_daily_note
  web.rs          — web_search, web_fetch
```

## New Tools

| Tool | Cave method | Category |
|------|------------|----------|
| `list_folders` | `cave.list_folders()` | navigation |
| `search_content` | new `cave.search_content()` | navigation |
| `move_note` | `cave.move_note()` | organization |
| `rename_note` | `cave.rename_note()` | organization |
| `create_folder` | `cave.create_folder()` | organization |
| `rename_folder` | `cave.rename_folder()` | organization |
| `move_folder` | `cave.move_folder()` | organization |
| `delete_folder` | `cave.delete_folder()` | organization |
| `open_daily_note` | `cave.open_daily_note()` | daily |

## Acceptance Criteria

- [ ] Tool source files reorganized by category (no behavior change)
- [ ] All new tools implemented and registered in build_toolset
- [ ] TOOL_CATALOGUE updated with all new tools
- [ ] build_tool_call_info extracts params for new tools
- [ ] Cave gets a search_content method for full-text body search
- [ ] All existing tests pass, new tools have unit tests
- [ ] cargo fmt && cargo clippy && cargo test pass