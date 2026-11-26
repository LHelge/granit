---
id: baa.cs
title: Embed zorite-editor as live-preview note editor with save
status: done
priority: P1
created: "2026-11-26T12:15:03.841943Z"
updated: "2026-11-26T13:32:40.096008Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.85
attempts: 1
---

## Summary

Show the selected note in `zorite-editor` with live-preview styling as the only note view, and save edits back through `Cave::save_note`. This is the core of the spike: it shows what the WYSIWYG-by-default experience feels like on real notes.

## Acceptance Criteria

- [ ] Selecting a note loads `Cave::read_note(slug)?.content` (body only, frontmatter already stripped) into the editor
- [ ] Live preview is on: headings sized, emphasis styled, markers hidden except under the caret, task checkboxes clickable, tables rendered as grids
- [ ] Edits are saved through `Cave::save_note(slug, &text)` on Cmd+S and on a debounce after the last change; switching notes flushes pending edits first
- [ ] The file's frontmatter is unchanged after a save (the cave layer re-attaches it)
- [ ] Editor fonts and colors come from the Catppuccin theme of the scaffold task, in both light and dark
- [ ] Fenced code blocks are highlighted through the editor's host highlighter hook (any tree-sitter or syntect backend is fine for the spike), or the gap is recorded

## Implementation Notes

- `zorite-editor` is a plain editor until a `SyntaxStyle` is supplied via `set_markdown_style`; live preview is opt-in.
- Host reads text with `text()` and listens for `Changed` events. Drive it on the main thread. Its layout cache is empty before first paint, so `bounds_for_offset` and IME queries return defaults until then.
- Only half the crate is documented on docs.rs; `API.md` in the repo (github.com/packetThrower/zorite-editor) and the Zorite app source are the references. The Zorite app is GPL-3.0: read it to understand usage, do not copy code from it.
- Current save behavior to mirror loosely: `src/app/editor/save_queue.rs` (debounced queue, flush on note switch).
- Leave image, math and Mermaid providers unset; relative images are checked in the evaluation task.

## Edge Cases

- Empty note; note with only a heading
- Note open while the agent or another process changes the file: out of scope, note the behavior only

## Testing

- Manual on a copy of the real cave
- `git diff` in the cave copy after typing and deleting one character shows no change