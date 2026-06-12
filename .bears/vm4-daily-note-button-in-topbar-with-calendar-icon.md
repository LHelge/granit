---
id: vm4
title: Daily note button in topbar with calendar icon
status: done
priority: P1
created: 2026-04-03T23:11:03.258131001Z
updated: 2026-04-03T23:19:19.560551541Z
tags:
- frontend
- ui
depends_on:
- xk2
parent: hh6
---

## Summary
Add a calendar icon button to the topbar, placed between the "Granit" title and the existing toggle buttons. Clicking it calls the `open_daily_note` IPC and sets the active note.

## Acceptance Criteria
- [ ] Calendar icon button (`LuCalendar` from `icondata_lu`) appears in the topbar
- [ ] Positioned to the right of the "Granit" `<span>`, before the sidebar/agent toggle buttons
- [ ] Only visible/enabled when a cave is open (check `ctx.config.get().active_cave.is_some()`)
- [ ] On click: calls `ipc::open_daily_note()`, sets `ctx.active_note`, refreshes note list
- [ ] Button matches existing toggle button styling (padding, hover, transitions)
- [ ] Tooltip: "Open daily note"

## Implementation Notes
- Location: `src/app/mod.rs`, inside the `<header>` element
- Add the button after the `<span>"Granit"</span>` but wrap it in a container or use flex gap
- Structure:
  ```rust
  <button
      class="p-1 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 transition-colors"
      on:click=move |_| {
          spawn_local(async move {
              match ipc::open_daily_note().await {
                  Ok(note) => {
                      ctx.active_note.set(Some(note));
                      // Refresh the note list to pick up newly created notes
                      if let Ok(notes) = ipc::list_notes().await {
                          ctx.notes.set(notes);
                      }
                  }
                  Err(e) => ctx.push_error(format!("Daily note: {e}")),
              }
          });
      }
      title="Open daily note"
  >
      <Icon icon=icondata_lu::LuCalendar width="1rem" height="1rem"/>
  </button>
  ```
- Only show when cave is open using `Show` or conditional rendering
- Refresh `ctx.notes` after calling in case a new note was created
- Also refresh `ctx.folders` in case the daily folder was newly created

## Edge Cases
- No cave open → button hidden or disabled
- Network/IO error → show via `ctx.push_error()`