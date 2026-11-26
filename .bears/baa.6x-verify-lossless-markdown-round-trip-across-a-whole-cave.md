---
id: baa.6x
title: Verify lossless markdown round-trip across a whole cave
status: in_progress
priority: P1
created: "2026-11-26T12:15:13.636400Z"
updated: "2026-11-26T13:32:40.131955Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.cs
attempts: 1
---

## Summary

Gate G2. Files on disk are the source of truth and agent tools edit them too, so the editor must never rewrite markdown it was not asked to change. Prove this on every note of a real cave, and find out which structured edits (checkbox, table cell, alert fold) rewrite more than the edited spot.

## Acceptance Criteria

- [ ] A test or a hidden `--roundtrip <cave>` mode loads each note body into an editor state and reads it back with `text()`: output equals input byte for byte for every note; mismatches are listed with slug and first differing offset
- [ ] Saving every note unedited through `Cave::save_note` leaves `git status` clean in a git-initialized copy of the cave (this also covers the frontmatter re-attach path)
- [ ] Typing one character in the middle of a paragraph produces a one-character diff
- [ ] Each of these is tried once and its diff recorded: toggle a task checkbox, edit one table cell, fold/unfold a `> [!NOTE]` alert, press Enter inside a list
- [ ] Line endings (CRLF notes if any exist), trailing whitespace, trailing newline and tabs survive

## Implementation Notes

- Work on a copy of the real cave with `git init && git add -A && git commit` so diffs are trivial to inspect.
- The editor documents that alert fold toggles are written back to the source (`> [!NOTE]-` / `+`); that is an intentional edit, but it must be limited to that marker.
- Table cell edits may re-pad the whole table. Record whether they do; re-padding one table on an explicit cell edit is tolerable, rewriting on load is not.
- If `save_note` itself changes bytes for an unedited body, that is an existing backend behavior: record it separately from editor findings.

## Edge Cases

- Notes with raw HTML, footnotes, nested lists with mixed markers, hard line breaks (two trailing spaces), Tera syntax in templates (only if templates are loaded)
- Very large note

## Testing

- The round-trip check itself is the deliverable; keep its output for the write-up
## Results (2026-11-26, zorite-editor 0.11.7)

Re-run: `granit-gpui --roundtrip <cave> [--save body|editor] [--out <file>]` (opens one hidden window; exit 0 identical, 1 mismatches, 2 error). Structured edits are `gpui::test`s in `granit-gpui/src/roundtrip.rs`.

Editor (set_text -> paint -> text()):
- Test cave at the time: 378 notes + 1 template, 378 identical, 1 mismatch. With 11 extra edge-case fixtures (now in the test cave under `Spike/Extra/`): 389 notes, 2 mismatches.
- Only failure: a `$$` fence sharing a line with words is split onto its own lines on load (`and $$x$$.` -> `and\n$$x$$\n.`), also inside inline code. An unedited note is never written, but the first edit anywhere in such a note writes the rewrite too.
- Lossless on load: CRLF, mixed endings, lone CR, BOM, tabs, trailing whitespace, missing/many trailing newlines, leading blank lines, raw HTML, entities, escapes, footnotes, mixed list markers, hard breaks, Tera syntax, setext headings, tables, a 6,900-line note.
- Single edits limited to the edited spot: one typed character (`+x`), task checkbox click (one byte), typing in a table cell (no re-padding), alert fold on `> [!NOTE]-` / `+` (the fold char only), Enter in an LF list (`\n- ` / `\n- [ ] `). A plain `> [!NOTE]` has no fold control.
- Not limited: Enter in a CRLF note writes a bare LF (the note ends up with mixed endings).

Backend `Cave::save_note` with an unedited body (existing behaviour, same in the Tauri app): 385 of 389 files change.
- `modified_at` re-stamped (or added); `created_at` reformatted when its fractional seconds are zero.
- Unknown frontmatter keys and YAML comments dropped; flow-style `tags: [a, b]` rewritten as a block; CRLF frontmatter rewritten with LF.
- Data loss: malformed frontmatter is removed entirely; a `---`-fenced block at the top of a note is treated as frontmatter and deleted, including when a body starts with its own `---\nkey: value\n---`.

Gate G2 as written ("saving every note without edits leaves every file byte-identical") fails on the backend alone; the editor's own failures are the `$$` split and LF on Enter in CRLF notes.
