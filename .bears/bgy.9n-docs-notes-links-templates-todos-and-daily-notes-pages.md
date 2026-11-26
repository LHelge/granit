---
id: bgy.9n
title: "docs: notes, links, templates, todos and daily notes pages"
status: open
priority: P2
created: "2026-11-26T23:42:04.885095400Z"
updated: "2026-11-26T23:42:04.885095400Z"
tags:
  - docs
---

## Summary

Fix the notes-and-writing pages: `notes-and-markdown.md`, `wiki-links.md`, `cave-rules.md`, `templates.md`, `todos.md` and `daily-notes.md`, all under `docs/content/wiki/`.

## Acceptance Criteria

- [ ] **wiki-links:** the anchor example no longer pairs a note named `Volvo` with `{#volvo}`. That combination refuses to open the cave (test `test_open_cave_rejects_anchor_colliding_with_note`, `src-tauri/src/cave/mod.rs`); use e.g. a `Brands` note with `## Volvo {#volvo}`. The page says that links and anchors match case-insensitively.
- [ ] **Rename behaviour** (wiki-links, cave-rules): renaming a note rewrites every `[[old]]` / `[[old|label]]` (`cave/notes.rs`); only deleting leaves broken links. Clicking a broken link creates the note (`src/app/editor/mod.rs`); a `YYYY-MM-DD` target becomes a daily note.
- [ ] **notes-and-markdown, new "Note header" section:** the editor never shows frontmatter (`read_note` returns the body only). The header edits it (`editor/writer.rs`, `editor/frontmatter.rs`): the title field (which renames the note), the icon picker, the favorite star, tags ("Add tag…"), and the presentation dropdown.
- [ ] **notes-and-markdown, new "Reader" section:** what the reader header shows (`editor/reader.rs`): tags, Created/Modified, word count and reading time, and the collapsible "Backlinks (N)" list. wiki-links links to this section instead of saying backlinks are only "tracked".
- [ ] **notes-and-markdown, frontmatter:** name the keys `created_at` / `modified_at` and say Granit sets them itself. Only `tags`, `icon` and `presentation` are copied from a template.
- [ ] **notes-and-markdown, markdown extensions:** add GitHub alerts (`> [!NOTE|TIP|IMPORTANT|WARNING|CAUTION]`, rendered in `markdown/render.rs`, with editor completion), footnotes, strikethrough and tables. This fixes the dangling "supported extensions below" reference.
- [ ] **notes-and-markdown, shortcuts:** add the missing ones from the shortcuts dialog (`src/app/keybinds.rs`): Cmd/Ctrl+E for edit mode, Tab / Shift+Tab to indent, `> [!` alert completion, Esc to close completion, and pasting a URL over a selection to make a link (`js/editor.ts`).
- [ ] **cave-rules:** only `.md` files become notes. A duplicate filename refuses to open the cave and the error names both paths (`DuplicateSlug`). Slugs match case-insensitively. Notes are renamed in the explorer or the title field, and links are rewritten.
- [ ] **templates:** name the UI. Notes come from right-click → "New note from template" / "New note here from template", and templates are created with "New note template" in the Templates tab. The body is *rendered* with Tera, not copied. An explicit template overrides the daily template for a date-named note.
- [ ] **todos:** checkboxes can be ticked in the Todo tab itself (`todo/groups.rs`). The tab groups items by note, opens a note on click, and keeps a collapsed "Completed (N)" section. Recognised syntax includes `*`, `+`, ordered markers and `[X]` (`cave/todos.rs`). The page drops its triple restatement and the task-list block it repeats from notes-and-markdown; it should be about half as long.
- [ ] **daily-notes:** notes are named `YYYY-MM-DD`. Any note created with such a slug (from the tree, a broken link, or agent `create_note`) goes to the daily folder with the daily template unless one is given (`cave/notes.rs`). The folder is created if missing. The button is the calendar's "Open daily note" icon. Clicking a date opens or creates that day's note, future dates included. The setting is Settings → Notes → "Daily note folder". The calendar sits above the explorer tabs and is not a tab. Trim the repeated "configurable folder" wording.
- [ ] Each page loses its "This page covers…" opener and any redundant "Related pages" list.

## Implementation Notes

- Load the `aphid-content` skill. Build from `docs/` with `aphid build -o <scratch>`.
- Every behavioural claim is verified against the cited code before it is written.