---
id: baa.ac
title: "Upstream: zorite-editor panics on a mouse event after set_text shortens the document"
status: open
priority: P2
created: "2026-11-26T15:01:11.760929Z"
updated: "2026-11-26T15:01:11.760929Z"
tags:
  - spike
  - gpui
  - editor
  - upstream
---

## Summary

`zorite-editor` 0.11.7 aborts the host app when a mouse event reaches the editor after `EditorState::set_text` replaced the document with a shorter one and before the next paint. Found by the owner on 2026-11-26 while trying the GPUI spike. Candidate for an upstream fix and PR to github.com/packetThrower/zorite-editor (the crate is MIT).

## The error

```
thread 'main' panicked at zorite-editor-0.11.7/src/lib.rs:5147:38:
index out of bounds: the len is 4 but the index is 20
...
panic in a function that cannot unwind
  gpui_macos::window::handle_view_event (gpui-pre-macos-0.3.12/src/window.rs:2944)
thread caused non-unwinding panic. aborting.
```

Seen with `cargo run -p granit-gpui -- ~/tmp/test_cave` (debug build) on macOS, gpui-pre 0.3.12, gpui-kit 0.10.0. The panic is raised inside an AppKit view event callback, which cannot unwind, so the process aborts and unsaved edits in the editor are lost.

## Cause (read from the source, confirmation by test pending in the spike)

- `EditorState::index_for_mouse_position` (`src/lib.rs` ~5127-5148) picks `row` by walking `self.wrapped` / `self.line_tops`, which hold the layout of the last paint, then indexes `self.line_starts()[row]`, which is computed from the current `self.content`.
- `EditorState::set_text` (`src/lib.rs` ~1235) replaces `content`, bumps `content_gen` and calls `cx.notify()`, but does not clear `wrapped`, `line_tops` or the other per-paint layout fields.
- Between `set_text` and the next paint the two disagree. With an old document of more than 20 rows and a new one of 4 lines, a pointer over old row 20 gives `line_starts()[20]` on a 4-element vector.
- Callers of `index_for_mouse_position`: mouse down (~3567, 3601, 3729), mouse move / drag (~3913), other click paths (~4097, 4105, 4140) and `character_index_for_point` (~5500, the IME / input-handler query).

Line numbers are for the 0.11.7 crate source as published.

## How a host triggers it

- Handling `EditorEvent::OpenWikiLink` by loading another note with `set_text`: the event is emitted from mouse down, and the mouse-up or a move arrives before the repaint.
- Switching documents from other UI (a file tree) while the pointer then crosses the editor before the next frame. A slow frame widens the window; a 6,900-line note takes seconds per frame in a debug build.

## Possible upstream fixes

- Clamp `row` to `line_starts().len() - 1` (and guard the other per-row lookups: `line_tops[row]`, `wrapped[row]`, `widget_rows`, `row_origin_x`, `bidi_map`, `source_col`) in `index_for_mouse_position`.
- Or invalidate the per-paint layout (`wrapped`, `line_tops`, hit rects such as `alert_fold_rects` / `heading_fold_rects`, task checkbox rects) in `set_text`, so the existing `self.wrapped.is_empty()` early return applies until the next paint.
- Check whether the same mismatch can occur after the editor's own large edits (select-all + delete, `replace_range`) followed by a mouse event before paint; the spike subagent is checking this and will record the answer in this task.

## Acceptance Criteria

- [x] Minimal reproduction outside Granit (a `gpui::test` or the crate's `examples/demo.rs`): paint a long document, `set_text` a short one, deliver a mouse move over a lower row without painting, observe the panic
- [x] Upstream issue filed with the reproduction
- [x] Fix prepared against the current upstream `main` (check first whether a newer release already fixes it) and sent as a PR
- [ ] Spike host-side workaround removed or kept deliberately once a fixed version is used

## Notes

- Host-side workaround being added in the spike: a fresh `EditorState` per opened note, whose empty layout takes the early-return path.
- Not a spike gate by itself, but it belongs in the write-up (`baa.ng`) under editor robustness for G4.
- Licence: only the MIT `zorite-editor` crate is involved; do not copy from the GPL-3.0 Zorite app.
## Confirmed in the spike (2026-11-26)

The diagnosis above is confirmed by headless tests in `granit-gpui/src/main.rs` (deliver a mouse-down/up through `window.dispatch_event` inside one `update_window`, before anything repaints; `simulate_click` does not reproduce it because the test window repaints first).

- Layout is only written in paint (`element.rs` ~2420-2448). The owner's crash at `lib.rs:5147` is the inline-image (widget) row branch of `index_for_mouse_position`; the normal branch is at ~5157. `checkbox_at` on mouse-down also indexes `line_starts()[row]` from the stale layout.
- Mouse-move reaches the function only during a drag-select (~3913); plain hover does not.
- The hazard is not limited to `set_text`. The editor does not refresh layout on its own edits either. Recorded as `should_panic` tests that will start failing once the crate is fixed:
  - `press_after_a_typed_edit_before_paint_panics_in_the_editor`: select all + type, then a press over a removed row -> index out of bounds at ~5157.
  - `press_after_replace_range_before_paint_panics_in_the_editor`: the same through `replace_range`.
  - `press_after_shortening_lines_parks_the_caret_out_of_range`: an edit that keeps the line count but shortens lines does not panic on the press; it parks the caret past the end of the text (caret 116, length 90) and the next keystroke panics at ~5405 ("end byte index 116 is out of bounds for string of length 90"). A delayed crash.
- So any line-removing or line-shortening edit (delete or cut of a multi-line selection, paste over a selection, undo, an accepted completion) followed by a press, right-click or drag-select before the next paint is exposed. Rare at normal frame times; likely when frames are slow (very large note, debug build). Read from the source, not tested: on a stale row a checkbox click may toggle the wrong line.

Host-side workaround in the spike: every opened note gets a fresh `EditorState` (test `press_before_the_switched_note_is_painted_does_not_panic`). This covers note switching only, not edits within a note. A further host mitigation (swallow mouse-down while `row_layout().len()` differs from the text's line count) was not built and would not cover the shortened-lines case.

A fix upstream therefore needs to cover edits as well as `set_text`: clamp `row` against `line_starts().len()` and the resulting offset against `content.len()`, or invalidate the per-paint layout on every content change.

## Upstream fix (2026-11-26)

- Checked first: upstream `main` equalled the published 0.11.7 and the repo had no issues, so the bug was neither fixed nor reported.
- Issue: packetThrower/zorite-editor#4. PR: packetThrower/zorite-editor#5, from branch `fix/stale-layout-input` of the owner's fork `LHelge/zorite-editor` (commit `0a37b2c`). Both open.
- The fix: `EditorState::layout_is_stale()` (`content_gen != last_paint_gen`). While it holds, left/right press, mouse move and the gutter grip press return early, Up/Down fall back to logical-line movement, and `character_index_for_point` returns `None`. A press is dropped for the stale frame rather than clamped, so it cannot act on the wrong line.
- Also found and covered: Up/Down right after an edit that removed the rows below the caret panicked the same way (`move_vertical`).
- Not covered, no reproduction: other keyboard paths that read the painted layout (bidi horizontal steps, Tab between table cells).
- The spike is pinned to the fork branch through `[patch.crates-io]` in the workspace `Cargo.toml`. The three `should_panic` tests are now `press_after_*_is_ignored` / `..._keeps_the_caret_in_the_text` and pass.

Left for the last criterion: when a release carries the fix, drop the `[patch.crates-io]` entry and decide whether the fresh-editor-per-note workaround stays.
