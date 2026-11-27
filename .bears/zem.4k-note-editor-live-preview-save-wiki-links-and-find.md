---
id: zem.4k
title: "Note editor: live preview, save, wiki-links and find"
status: done
priority: P1
created: "2026-11-26T21:09:38.830001Z"
updated: "2026-11-27T01:05:54.282383766Z"
tags:
  - gpui
  - port
  - frontend
  - editor
depends_on:
  - zem.4b
  - zem.ry
attempts: 1
---

## Summary

Embed `zorite-editor` as the only note view: load and save through the core crate, autosave, wiki-link navigation and `[[` completion, broken-link marking, find in note, and relative images. Most of this exists in the spike and should be ported with tests, not rewritten.

## Acceptance Criteria

- [ ] Opening a note shows it in live preview; edits autosave and Cmd-S saves; unsaved edits are written before switching notes or quitting
- [ ] Each opened note gets a fresh `EditorState`, and one tree-sitter highlighter per language is shared across editors (both spike findings, see `baa.ng`)
- [ ] `[[note]]`, `[[note|label]]` and `{#id}` anchors navigate on click through `Cave::resolve_link`; anchors scroll to the heading
- [ ] `[[` completion of slugs and anchors, with Up/Down/Enter/Tab/Escape, not triggered inside code
- [ ] Broken links are visibly marked
- [ ] Find in note (`find_in_source` / `set_search`) with a minimal search bar
- [ ] Relative images in a note display, resolved against the note's directory
- [ ] The `$$` fence split and the bare LF on Enter in a CRLF note (spike `baa.6x`) are fixed, upstream in `zorite-editor` if the cause is there

## Implementation Notes

- Spike sources on `spike/gpui`: `granit-gpui/src/note_editor.rs`, `save.rs`, `link_nav.rs`, `wiki_links.rs`, and the editor parts of `main.rs`. Verdicts and API notes are in `baa.cs` and `baa.9s`.
- Gaps from the spike to close: indented code blocks and multi-line inline code are not detected as code for completion; clicking a broken link does nothing (decide: create the note?).
- Known editor gap the owner accepted: a typed ```` ``` ```` fence turns into a rendered code card at once. If upstream ever offers raw editing of fences, adopt it.
- Task checkboxes, alerts, tables, footnotes and formatting shortcuts come with the editor; check the shortcut set against `editingKeymap` in `js/editor.ts`.

## Testing

- Port the spike's `gpui::test`s for navigation, completion, save and the stale-layout presses
- Round-trip check over the edge-case fixtures: open and save without edits is byte-identical (with the backend fix from the save task)