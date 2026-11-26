---
id: baa.6x
title: Verify lossless markdown round-trip across a whole cave
status: open
priority: P1
created: "2026-11-26T12:15:13.636400Z"
updated: "2026-11-26T12:15:13.636400Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.cs
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