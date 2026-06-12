---
id: hh6
title: Daily note button with auto-create/open
type: epic
status: done
priority: P1
created: 2026-04-03T23:07:56.737719519Z
updated: 2026-04-03T23:21:13.884816318Z
---

## Scope

Add a "daily note" button to the topbar (right of the "Granit" title) that opens or creates today's note. The daily note lives in a configurable folder (default: `Daily`) and uses the date as filename (e.g. `2026-04-04.md`).

## Acceptance Criteria

- [ ] A calendar icon button appears in the topbar, immediately right of the "Granit" title
- [ ] Clicking the button opens today's daily note (`YYYY-MM-DD.md`) in the editor
- [ ] If the note doesn't exist, it is created first (including the folder if needed)
- [ ] The daily note folder name is configurable via `daily_note_folder` in AppConfig (default: `"Daily"`)
- [ ] Config is layered: global default can be overridden per-cave
- [ ] Backend unit tests cover create-or-open logic, folder creation, and config defaults
- [ ] `cargo fmt && cargo clippy && cargo test` pass