---
id: jwr
title: Add move_note and rename_note tools
status: done
priority: P1
created: 2026-04-05T11:04:28.387917933Z
updated: 2026-04-05T11:25:29.983288604Z
tags:
- agent
depends_on:
- fmh
parent: fm9
---

## Summary
Add organization tools for notes: `move_note` and `rename_note`. Cave API already supports both.

## Implementation (in `organization.rs`)

### move_note
- `MoveNoteArgs { slug: String, destination: Option<String> }`
- Calls `cave.move_note(slug, destination.map(Path::new))`
- Returns new slug and relative path

### rename_note  
- `RenameNoteArgs { slug: String, new_name: String }`
- Calls `cave.rename_note(old_slug, new_name)`
- Returns new slug and relative path

## Registration
- Add both to `build_toolset`, `TOOL_CATALOGUE`
- Add slug extraction to `build_tool_call_info`

## Acceptance Criteria
- [ ] Agent can move a note to a different folder
- [ ] Agent can rename a note in-place