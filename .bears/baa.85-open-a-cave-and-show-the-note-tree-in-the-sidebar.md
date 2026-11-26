---
id: baa.85
title: Open a cave and show the note tree in the sidebar
status: open
priority: P1
created: "2026-11-26T12:14:53.041330Z"
updated: "2026-11-26T12:14:53.041330Z"
tags:
  - spike
  - gpui
depends_on:
  - baa.7f
---

## Summary

Let the spike binary open a real cave through the existing backend `Cave` type, called in-process, and show notes and folders as a tree in the sidebar. Selecting a note emits its slug to the main area (the editor task consumes it).

## Acceptance Criteria

- [ ] `cargo run -p granit-gpui -- <cave-path>` opens that cave; a missing or invalid path shows the `CaveError` text in the window instead of panicking
- [ ] The sidebar shows the folder/note tree from `Cave::list_folders` and `Cave::list_notes`, sorted like the current explorer, with expand/collapse
- [ ] Clicking a note selects it and the main area shows its slug (placeholder until the editor lands)
- [ ] Scrolling the tree of the real cave is smooth (virtualized list or tree component)

## Implementation Notes

- Backend access for the spike: in `src-tauri/src/lib.rs` change `mod cave;` and `mod markdown;` to `pub mod`, and have `granit-gpui` depend on `granit_lib = { path = "../src-tauri", package = "granit" }`. This drags Tauri into the spike's compile; that is accepted for the spike. Extracting a Tauri-free `granit-core` crate is port work, not spike work.
- The only Tauri uses inside `cave/` and `agent/` are `From<tauri::Error>` in `src-tauri/src/cave/error.rs` and one `tauri::async_runtime::spawn` in `src-tauri/src/agent/vectordb.rs`; note anything else that surfaces, since it sizes the later core extraction.
- Use `Cave::open(path)` then `ensure_scanned()` (see `with_shared_cave` in `src-tauri/src/commands/state.rs` for the lock-then-scan pattern). Hold the cave in a GPUI entity/global; run the scan off the main thread with GPUI's background executor if it takes noticeable time.
- The existing tree shape logic lives in `src/app/explorer/tree_view/tree_model.rs` (224 lines) and can be ported as plain Rust.

## Edge Cases

- A cave with a duplicate anchor refuses to open (`CaveError::DuplicateAnchor`); show the message.
- Empty cave: empty tree, no crash.

## Testing

- Manual against a copy of the real cave (never the live one; the round-trip task writes to it)
- `cargo test -p granit` still passes after the visibility change