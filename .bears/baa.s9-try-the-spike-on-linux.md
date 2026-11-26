---
id: baa.s9
title: Try the spike on Linux
status: open
priority: P2
created: "2026-11-26T21:13:42.141673Z"
updated: "2026-11-26T21:13:42.141673Z"
tags:
  - spike
  - gpui
  - linux
---

## Summary

The spike was only built and tried on macOS. Before the port starts, the owner wants to run it on their Linux machine to see that GPUI, GPUI Kit and zorite-editor build and feel right there too. Results feed `zem.nb` (Windows and Linux builds) and the port's go decision for Linux.

## Acceptance Criteria

- [ ] `cargo build --release -p granit-gpui` succeeds on Linux; note the distribution, desktop (X11 or Wayland) and GPU
- [ ] `cargo test -p granit-gpui` passes, including the stale-layout tests (`press_after_*_is_ignored`, `press_after_shortening_lines_keeps_the_caret_in_the_text`), which were only run on macOS
- [ ] The app opens a test cave, shows the tree, and opens, edits and saves notes
- [ ] Wiki-link clicks and `[[` completion work
- [ ] Keyboard: Swedish layout, dead keys (´ ¨ ^ ~), and the compose key or IME if used
- [ ] Scrolling the 6,910-line note and resizing the window feel smooth
- [ ] `cargo run --release -p granit-gpui --bin stream_smoke` renders the streamed markdown
- [ ] Rough numbers, if easy: warm start (`GRANIT_T0_NS=$(date +%s%N) GRANIT_OPEN="Test note" target/release/granit-gpui <cave>`) and idle memory (`ps -o rss` or `smem`)
- [ ] Anything that breaks or looks wrong is listed here as a gap or a blocker

## Implementation Notes

- System packages GPUI needs (Debian/Ubuntu names, from zorite-editor's CI): `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb1-dev libxcb-randr0-dev libxcb-xkb-dev libxcb-cursor-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev libfontconfig1-dev libfreetype-dev pkg-config`, plus a working Vulkan driver.
- `granit_lib` (src-tauri) is linked by the spike, so its Linux build dependencies (webkit2gtk and friends) are needed too, unless the core crate is extracted first.
- zorite-editor comes from the fork branch through `[patch.crates-io]` in the workspace `Cargo.toml`; cargo fetches it over HTTPS.
- Test cave: never a live cave. Rebuild one as described in the "Handover status" section of the epic (`granit-gpui/fixtures/gen_cave.py` plus `granit-gpui/fixtures/extra/*.md`).

## Testing

- Manual; this task is the test