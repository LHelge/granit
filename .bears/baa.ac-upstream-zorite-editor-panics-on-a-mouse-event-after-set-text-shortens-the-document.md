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

- [ ] Minimal reproduction outside Granit (a `gpui::test` or the crate's `examples/demo.rs`): paint a long document, `set_text` a short one, deliver a mouse move over a lower row without painting, observe the panic
- [ ] Upstream issue filed with the reproduction
- [ ] Fix prepared against the current upstream `main` (check first whether a newer release already fixes it) and sent as a PR
- [ ] Spike host-side workaround removed or kept deliberately once a fixed version is used

## Notes

- Host-side workaround being added in the spike: a fresh `EditorState` per opened note, whose empty layout takes the early-return path.
- Not a spike gate by itself, but it belongs in the write-up (`baa.ng`) under editor robustness for G4.
- Licence: only the MIT `zorite-editor` crate is involved; do not copy from the GPL-3.0 Zorite app.