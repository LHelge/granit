---
id: ee7
title: Add folder management tools
status: done
priority: P2
created: 2026-04-05T11:04:35.264891377Z
updated: 2026-04-05T11:35:48.035312584Z
tags:
- agent
depends_on:
- fmh
parent: fm9
---

## Summary
Add folder management tools: `create_folder`, `rename_folder`, `move_folder`, `delete_folder`. All cave methods exist.

## Implementation (in `organization.rs`)

### create_folder
- `CreateFolderArgs { path: String }`
- Calls `cave.create_folder(Path::new(&path))`

### rename_folder
- `RenameFolderArgs { path: String, new_name: String }`
- Calls `cave.rename_folder(Path::new(&path), &new_name)`

### move_folder
- `MoveFolderArgs { source: String, destination: String }`
- Calls `cave.move_folder(Path::new(&source), Path::new(&dest))`

### delete_folder
- `DeleteFolderArgs { path: String }`
- Calls `cave.delete_folder(Path::new(&path))`

## Registration
- Add all to `build_toolset`, `TOOL_CATALOGUE`
- Add path extraction to `build_tool_call_info`

## Acceptance Criteria
- [ ] Agent can create, rename, move, and delete folders
- [ ] All four tools registered and visible in settings