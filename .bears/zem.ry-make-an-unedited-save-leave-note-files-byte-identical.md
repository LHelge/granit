---
id: zem.ry
title: Make an unedited save leave note files byte-identical
status: done
priority: P1
created: "2026-11-26T21:09:11.675360Z"
updated: "2026-11-27T00:12:05.793266422Z"
tags:
  - gpui
  - port
  - backend
  - core
depends_on:
  - zem.4q
attempts: 1
---

## Summary

Spike gate G2 failed mostly on the backend, not the editor: `Cave::save_note` rewrites frontmatter on every save, so an unedited save changed 385 of 389 notes in the test cave, and some notes lose data. Fix the save path so frontmatter is only changed where Granit means to change it. This also affects today's Tauri app.

## Acceptance Criteria

- [ ] Saving a note whose body did not change does not write the file at all, or writes identical bytes
- [ ] When the body changed, frontmatter is preserved apart from the fields Granit owns (`modified_at`, and only if that is still wanted); unknown keys, key order, YAML comments, flow-style `tags: [a, b]`, quoting and line endings survive
- [ ] `created_at` is never reformatted (today it is when its fractional seconds are zero)
- [ ] A note with malformed frontmatter keeps it verbatim instead of losing it
- [ ] A body that starts with its own `---\nkey: value\n---` block is not mistaken for frontmatter and deleted
- [ ] CRLF frontmatter stays CRLF

## Implementation Notes

- Results and the failing cases are in `baa.6x` ("Results") and the fixtures in `granit-gpui/fixtures/extra/` on `spike/gpui` (copy them over). `granit-gpui --roundtrip <cave> --save body` on the spike branch re-runs the whole-cave check.
- Likely approach: keep the raw frontmatter text from `read_note` and splice only owned fields on save, instead of re-serializing the parsed struct with `serde_yml`.
- Decide with the owner whether `modified_at` should still be stamped on save, or only when the body changed.

## Testing

- Round-trip tests over the edge-case fixtures: unedited save is byte-identical; one body edit changes only the body (and `modified_at` if kept)