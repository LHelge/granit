---
id: baa.q8
title: Evaluate editing feel against a checklist
status: done
priority: P2
created: "2026-11-26T12:15:35.642615Z"
updated: "2026-11-26T21:06:19.035284Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.cs
---

## Summary

Gate G4. Use the spike for real writing for a while and record, item by item, whether the live-preview editor is good enough to be the only note view. This is the owner's hands-on evaluation; the result is a pass / gap / blocker table.

## Acceptance Criteria

Each item is marked pass, gap (acceptable for now), or blocker:

- [ ] Swedish keyboard: å ä ö, dead keys (´ ¨ ^ ~), Option-key symbols, emoji picker
- [ ] Caret movement and selection: word/line jumps, Shift-select, double/triple click, drag select across styled blocks
- [ ] Undo/redo grouping feels natural
- [ ] Clipboard: copy/cut/paste plain text; pasting a URL over a selection (current editor wraps it as a link)
- [ ] Lists: Enter continues, Enter on empty item ends, Tab/Shift-Tab indent
- [ ] Task checkboxes: click toggles; a keyboard toggle exists or is a gap
- [ ] Formatting shortcuts: bold, italic, inline code, link (current keymap is in `js/editor.ts`, `editingKeymap`)
- [ ] Headings, block quotes, GitHub alerts, tables, fenced code, horizontal rules, footnotes render and stay editable
- [ ] Markers reappearing under the caret feels predictable, not jumpy (layout shift when entering/leaving a line)
- [ ] Find in note (the editor exposes `find_in_source` / `set_search`; a minimal search bar is enough)
- [ ] Relative images display when an image provider is wired to the note's directory
- [ ] Large note (5,000+ lines): typing and scrolling stay smooth
- [ ] Scrolling and resize feel compared with the current app
- [ ] Spell check: note whether the OS spell checker can be hooked up (`on_suggest`), not required

## Implementation Notes

- Wire only what is cheap; a missing shortcut that is obviously addable in host code is a gap, not a blocker. A blocker is something the editor crate prevents.
- Keep the table in the epic write-up task.

## Testing

- Manual; this task is the test

## Owner verdict (2026-11-26)

Overall: editing "feels great". The owner tried the spike for real writing but did not mark the checklist item by item, so the per-item table is not filled in.

| Item | Verdict |
|---|---|
| Fenced code blocks (```` ``` ````) | gap: a fence turns into a rendered code card as soon as it is typed; the owner would prefer to keep editing it as markdown, but can get used to it |
| Everything else on the checklist | no gap reported; not checked item by item |

G4 passes: no blockers.
