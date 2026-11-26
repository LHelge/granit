---
id: baa.9s
title: Wiki-link navigation and [[ completion in the editor
status: open
priority: P1
created: "2026-11-26T12:15:24.743266Z"
updated: "2026-11-26T12:15:24.743266Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.cs
---

## Summary

Gate G3. Wiki-links are Granit's core navigation, and the current CodeMirror editor offers `[[` completion and broken-link styling. `zorite-editor` renders wiki-links and emits `OpenWikiLink`, but no completion-provider API is documented, so this task finds out whether completion can be built on top of it.

## Acceptance Criteria

- [ ] `[[note]]` and `[[note|label]]` render as links in live preview
- [ ] Clicking a wiki-link (handle `OpenWikiLink`) resolves through `Cave::resolve_link` and opens the target note; a `note#anchor` result opens the note and scrolls to the heading
- [ ] Clicking a regular URL (`OpenLink`) opens it in the system browser
- [ ] Typing `[[` shows a popup anchored at the caret with note slugs and anchors (`Cave::list_notes`, `Cave::list_anchors`), filtered as the user types; Up/Down selects, Enter or Tab inserts and closes with `]]`, Escape dismisses
- [ ] Broken links are visually distinct from resolved ones (`Cave::list_broken_links`), or it is recorded that the editor cannot style them differently
- [ ] Verdict written down: completion is (a) supported by the crate, (b) buildable in host code, or (c) needs an upstream change or fork, with the reason

## Implementation Notes

- Current behavior to match: `wikiLinkCompletionSource`, `applyWikiLink` and `buildLinkDecorations` in `js/editor.ts` (lines ~603-890).
- Likely host-side approach: listen to `Changed` / `SelectionChanged`, detect an unclosed `[[` before the caret in `text()`, position a GPUI Kit popover or list using the editor's `bounds_for_offset`, and intercept Up/Down/Enter/Escape with a key context that takes precedence while the popup is open. Whether key interception is possible without patching the editor is the main unknown.
- The Zorite app itself has `[[` completion, so some path exists; find out whether it lives in the editor crate or the (GPL) app. Do not copy app code.
- Heading anchors: only headings with `{#id}` are targets; see the "Heading anchors" section of `CLAUDE.md`.

## Edge Cases

- `[[` inside inline code or a fenced code block must not trigger completion (current editor checks `inCodeContext`)
- Slugs containing spaces and non-ASCII characters (å, ä, ö)

## Testing

- Manual on the cave copy; record a short list of what works and what does not for the write-up