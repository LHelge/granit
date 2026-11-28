---
id: zem.vs
title: Track zorite-editor and GPUI Kit upstream
status: open
priority: P3
created: "2026-11-26T21:10:43.332797Z"
updated: "2026-11-26T21:10:43.332797Z"
tags:
  - gpui
  - port
  - deps
depends_on:
  - zem.4k
---

## Summary

The port starts on a pinned, slightly old stack: gpui-kit 0.10 (0.11 pins a gpui-pre that zorite-editor 0.11.7 did not build with) and zorite-editor from the owner's fork branch. Move to released versions when upstream allows.

## Acceptance Criteria

- [ ] When packetThrower/zorite-editor#5 is in a release, the `[patch.crates-io]` fork pin is removed (closes the last criterion of `baa.ac`); decide then whether the fresh-editor-per-note workaround stays
- [ ] When packetThrower/zorite-editor#6 (cached `line_starts`; layout was quadratic in note size, see `baa.s9`) is in a release, check that the 6,907-line fixture note scrolls smoothly. Until then the fork pin does not carry it, so large notes are slow
- [ ] When a zorite-editor release builds on the gpui-pre that a newer gpui-kit pins, move both, with one gpui-pre in `cargo tree`
- [ ] Note whether the official `gpui` crate (0.2.2 on 2026-11-26) has caught up and whether to switch from `gpui-pre`
- [ ] Upstream any editor fixes found during the port (e.g. the `$$` fence split or CRLF Enter, if they are editor bugs)

## Implementation Notes

- Check crates.io for `zorite-editor`, `gpui-kit`, `gpui-pre` and `gpui` before starting.
- After rebasing the fork branch, `cargo update -p zorite-editor` refreshes the lockfile.

## Testing

- Full test suite and a manual run of the app after each bump
## Upstream items found during the port (2026-11-27)

Workarounds in `granit-app/src/note_view/` that an upstream zorite-editor change would remove:

- `$$` split on load: `normalize_loaded` (`src/lib.rs` ~387, from `set_text` ~1235 / `with_text`) runs `normalize_math_fences`. Needs a non-normalizing load (`set_text_raw` or an option) or `EditorState::clear_history()`; removes `load_text`'s `replace_range` and the `undo_floor` guard.
- `$$` split while typing: `apply_auto_replace` (~1434) calls `normalize_math_at` (~4223). Needs a switch (`set_math_normalization(false)`); removes the always-firing empty `set_auto_replace` rule.
- CRLF: `EditorState::newline` (~2433-2514) inserts bare `\n` and paste (~2945) converts CRLF to LF. Needs line-ending preservation; the host-side `save::LineEnding` conversion can stay.
- No selection setter: `NoteView::select_range` fakes one with `replace_and_mark_text_in_range` + `unmark_text` and swallows the `Changed` echo (`selection_echo`). Needs a public `select_range(range)`. Re-check this workaround whenever the zorite pin moves: if the echo stops firing, the next real edit is swallowed.
- No multi-cursor: select-next moves the single selection instead of adding one.
- Optional: route Copy/Cut through `set_clipboard_writer` to put HTML on normal copies (`granit-app/src/clipboard.rs`).
