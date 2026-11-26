---
id: bgy.u5
title: "docs: explorer, getting started and home pages"
status: open
priority: P2
created: "2026-11-26T23:42:04.903797970Z"
updated: "2026-11-26T23:42:04.903797970Z"
tags:
  - docs
---

## Summary

Fix `docs/content/wiki/explorer.md`, `docs/content/wiki/getting-started.md` and `docs/content/home.md`.

## Acceptance Criteria

- [ ] **explorer:**
  - The calendar sits above the tab bar and is not a tab (`src/app/explorer/mod.rs`). It shows week numbers, and clicking a date opens or creates that day's note. Daily-note details link to daily-notes.
  - The Tags tab groups notes by tag in collapsible groups and does not filter (`explorer/tags.rs`).
  - There is an Agent tab.
- [ ] **explorer, context menus:** list the labels as they appear in `tree_view/context_menu.rs`:
  - on a note: Rename, Delete note;
  - on a folder: New note here, New note here from template, New folder here, Rename, Delete folder;
  - on the empty background: New note, New note from template, New folder.
- [ ] **explorer, warning:** deleting a note or folder is immediate and permanent, with no confirmation (`remove_file` / `remove_dir_all`).
- [ ] **explorer, other:**
  - Ctrl/Cmd+click and Shift+click select several notes, which can be dragged together (`tree_view/mod.rs`).
  - Search is a case-insensitive substring match on note bodies (`cave/search.rs`).
  - Renaming a note rewrites the links to it.
- [ ] **getting-started, the `.granit/` tree:** list what opening a cave actually creates: `config.yml`, `agent/system.md`, and `presentations/` with two defaults. `templates/` appears with the first template, and `embeddings.bin` / `backup.key` appear later. Alternatively, link to cave-rules for this.
- [ ] **getting-started, the tour:**
  - The tab list has 7 tabs, including Agent, and the Templates tab also holds presentation templates.
  - The calendar is above the tabs.
  - The cave selector is the folder button at the bottom of the explorer ("No cave open").
  - The title bar has the Toggle sidebar / Toggle agent buttons, panel edges drag to resize, and Cmd/Ctrl+E enters edit mode.
  - A cave with duplicate filenames or anchors refuses to open.
- [ ] **getting-started, length:** cut the `.granit/` and tour sections by about a third where they repeat cave-rules, explorer and configuration.
- [ ] **home:**
  - The agent can *list* templates, not manage them (its only template tool is `list_templates`).
  - Todos can be toggled from the Todo tab.
  - Add short sections, with links, for presentations, Mermaid diagrams, encrypted backups, and agent skills and tasks.

## Implementation Notes

Load the `aphid-content` skill. Verify each UI label in the `view!` macros before writing it.