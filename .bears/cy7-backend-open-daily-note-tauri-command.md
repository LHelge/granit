---
id: cy7
title: Backend open_daily_note Tauri command
status: done
priority: P1
created: 2026-04-03T23:09:51.519336185Z
updated: 2026-04-03T23:18:05.187907900Z
tags:
- backend
- cave
depends_on:
- kf9
parent: hh6
---

## Summary
Add a backend Tauri command `open_daily_note` that creates or opens today's daily note. This is a single command that encapsulates the full logic: ensure folder exists → check if note exists → create if not → return the full `Note`.

## Acceptance Criteria
- [ ] New `#[tauri::command] fn open_daily_note(state)` in `src-tauri/src/lib.rs`
- [ ] Reads `daily_note_folder` from config to determine target folder
- [ ] Computes today's date as `YYYY-MM-DD` for the note slug/filename
- [ ] If the daily folder doesn't exist, creates it via `cave.create_folder()`
- [ ] If the note doesn't exist, creates it via `cave.create_note()` in that folder
- [ ] Always reads and returns the full `Note` (with rendered HTML) — same as `read_note`
- [ ] Returns `CaveError` variants for failures (no cave open, IO errors)
- [ ] Registered in `generate_handler![]`

## Implementation Notes
- Add a method `Cave::open_daily_note(folder: &str)` in `src-tauri/src/cave/mod.rs` that:
  1. Gets today's date: `chrono::Local::now().format("%Y-%m-%d").to_string()` (add `chrono` dep)
  2. Checks if folder exists, creates if not
  3. Checks if slug exists in `self.notes`, if not calls `self.create_note(&date_str, Some(folder_path))`
  4. Calls `self.read_note(&slug)` and returns the result
- The Tauri command is thin: reads config for `daily_note_folder`, delegates to `cave.open_daily_note()`
- Need `chrono` crate with `clock` feature for `Local::now()`

## Edge Cases
- No cave open → return appropriate error
- Folder name with nested path (e.g. `"Notes/Daily"`) should work
- Note already exists → just open it, no error

## Testing
- Unit test: create daily note in temp cave, call again same day → returns same note
- Unit test: verify correct folder and filename format