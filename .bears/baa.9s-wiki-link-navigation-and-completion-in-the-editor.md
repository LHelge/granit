---
id: baa.9s
title: Wiki-link navigation and [[ completion in the editor
status: in_progress
priority: P1
created: "2026-11-26T12:15:24.743266Z"
updated: "2026-11-26T13:55:36.180387Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.cs
attempts: 1
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
## Results (2026-11-26, zorite-editor 0.11.7)

- Verdict on `[[` completion: (b) buildable in host code, built in the spike without patching the editor. The crate has no completion API; the Zorite app's completion lives in the GPL app (`src/slash.rs`, `src/ui/slash_menu.rs`), not the crate. The host uses `cursor()`, `text()`, `Changed`/`SelectionChanged`, window-space `bounds_for_offset` for anchoring, `capture_action` on an ancestor div for the editor's public `Up`/`Down`/`Newline`/`Indent`/`Dismiss` actions, and `replace_range` for the insert (which emits no `Changed`, so the host records the edit).
- Navigation: `OpenWikiLink` is shared by `[[links]]`, `#tags`, property pills and block refs, so the host maps the last mouse press to a text offset (`character_index_for_point`) and navigates only when it is inside a real `[[...]]` outside code; the target is resolved through `Cave::resolve_link`. Anchor targets open the note and scroll the host scroll view to the `{#id}` heading. `OpenLink` goes to `cx.open_url` for http(s) and mailto only.
- Broken links: `SyntaxStyle` has one link colour, so they cannot be coloured differently; the host marks them with the editor's diagnostics (red wavy underline). Side effect: right-clicking one opens the spell menu.
- Differences from the CodeMirror editor: a plain click follows a link (no Cmd needed); the popup opens only on typing; typing `|` closes it; ranking is exact > prefix > word start > substring > subsequence.
- Gaps: code detection ignores indented code blocks and multi-line inline code; clicking a broken link does nothing (no create-note action); link scanning runs over the whole text on every change (not measured on large notes).
- Headless evidence: simulated clicks open plain, labelled and non-ASCII targets; an anchor click lands the heading within 100 px of the viewport top; `[[` + typing filters, Up/Down/Enter/Tab/Escape work. Rendering, popup position and the underline are not verified by eye.
