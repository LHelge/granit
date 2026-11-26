---
id: baa.s9
title: Try the spike on Linux
status: done
priority: P2
created: "2026-11-26T21:13:42.141673Z"
updated: "2026-11-26T23:12:16.473619503Z"
tags:
  - spike
  - gpui
  - linux
attempts: 1
---

## Summary

The spike was only built and tried on macOS. Before the port starts, the owner wants to run it on their Linux machine to see that GPUI, GPUI Kit and zorite-editor build and feel right there too. Results feed `zem.nb` (Windows and Linux builds) and the port's go decision for Linux.

## Acceptance Criteria

- [x] `cargo build --release -p granit-gpui` succeeds on Linux; note the distribution, desktop (X11 or Wayland) and GPU
- [ ] `cargo test -p granit-gpui` passes, including the stale-layout tests (`press_after_*_is_ignored`, `press_after_shortening_lines_keeps_the_caret_in_the_text`), which were only run on macOS
- [ ] The app opens a test cave, shows the tree, and opens, edits and saves notes (opens the cave, shows the tree and opens notes; edit/save not yet confirmed)
- [ ] Wiki-link clicks and `[[` completion work
- [ ] Keyboard: Swedish layout, dead keys (´ ¨ ^ ~), and the compose key or IME if used
- [ ] Scrolling the 6,910-line note and resizing the window feel smooth (scrolling smooth after the `line_starts` fix below; resizing not yet tried)
- [ ] `cargo run --release -p granit-gpui --bin stream_smoke` renders the streamed markdown
- [ ] Rough numbers, if easy: warm start (`GRANIT_T0_NS=$(date +%s%N) GRANIT_OPEN="Test note" target/release/granit-gpui <cave>`) and idle memory (`ps -o rss` or `smem`)
- [ ] Anything that breaks or looks wrong is listed here as a gap or a blocker

## Results (2026-11-26, first session)

### Environment

- Arch Linux, kernel 7.3.5, Hyprland (Wayland), one 3840x1600 monitor at 120 Hz, scale 1
- AMD Radeon RX 9070 XT (Navi 48, RDNA4), Mesa 26.3.1 RADV. RADV prints `radv is not a conformant Vulkan implementation, testing use only` at startup (the GPU is not yet on Mesa's conformance list); harmless so far
- All build libraries were already installed (xkbcommon, wayland, xcb, fontconfig, freetype, webkit2gtk-4.1, Vulkan). The release build succeeded first time

### Blocker found and fixed: quadratic `line_starts` in zorite-editor

- Symptom: scrolling `Spike/Long note for scrolling` (6,907 lines, 226 KB) was very slow and laggy, and switching notes in the tree took several seconds. Idle CPU was 0%, and all CPU time was on the main thread.
- Cause (from `perf`): 95.5% of CPU was in `EditorState::line_starts`, which rescanned the whole document and allocated a new `Vec` on every call. It has about 60 call sites, and two loops in the element layout (`element.rs`, heading chevrons and per-line hitboxes) call `line_end(i)` -> `line_starts()` for every line of the document on every frame. For this note that is about 3 GB of byte scanning per frame. The bug is not Linux-specific; it probably also explains the "~1 s to load in release" seen on macOS.
- Fix: `line_starts()` is cached per `content_gen` (the same pattern as `scan_cache`) and returns `Rc<[usize]>`. Every write to `content` was checked to bump `content_gen` first; `with_text` now bumps it too. Includes a new test, `cached_line_starts_follow_edits_undo_and_set_text`. It lives on branch `perf/cache-line-starts`, off `main`, in the owner's fork (`~/dev/zorite-editor`). The owner commits it and opens the upstream PR, separate from #5. It applies cleanly on top of `fix/stale-layout-input` (78 tests pass there).
- Measured (release build, `perf record -e cycles:u`, scrolling the long note and clicking around the tree):

| | Before | After |
|---|---|---|
| Session length | 48 s | 66 s |
| CPU samples (999 Hz) | 37,172 | 3,618 |
| Share of one core busy | ~77% | ~5.5% |
| `line_starts` share | 95.5% | 0.21% |

  After the fix the top cost is `zorite_editor::element::shape_document` (21% of the much smaller total). Owner: "much smoother now".
- Upstream: packetThrower/zorite-editor#6, separate from #5. Owner's decision: granit stays pinned to `fix/stale-layout-input` (the panic fix matters more), and large notes are avoided until #6 is released. No combined fork branch. Tracked in `zem.vs`.

### Profiling on this machine

- `perf_event_paranoid=2` and `ptrace_scope=1`: attaching to a running process does not work, and only user-space events with user-only call chains do. Launch the app under perf: `perf record -e cycles:u --user-callchains -F 999 --call-graph dwarf,16384 -o <file> -- target/release/granit-gpui <cave>`. The release binary has no debug info, so call stacks do not resolve, but self-time per symbol is enough to find hot spots.

### Not tried

- The owner called the Linux try-out done after the fix (2026-11-27). The unticked criteria above were not checked separately on Linux: `cargo test -p granit-gpui` (stale-layout tests), editing and saving, wiki-link clicks and `[[` completion, Swedish keyboard and dead keys, window resizing, `stream_smoke`, warm start and idle memory, and an X11 run (unset `WAYLAND_DISPLAY`). Cover them in `zem.nb` when the port's Linux build is set up.

## Implementation Notes

- System packages GPUI needs (Debian/Ubuntu names, from zorite-editor's CI): `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb1-dev libxcb-randr0-dev libxcb-xkb-dev libxcb-cursor-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev libfontconfig1-dev libfreetype-dev pkg-config`, plus a working Vulkan driver.
- `granit_lib` (src-tauri) is linked by the spike, so its Linux build dependencies (webkit2gtk and friends) are needed too, unless the core crate is extracted first.
- zorite-editor comes from the fork branch through `[patch.crates-io]` in the workspace `Cargo.toml`; cargo fetches it over HTTPS.
- Test cave: never a live cave. Rebuild one as described in the "Handover status" section of the epic (`granit-gpui/fixtures/gen_cave.py` plus `granit-gpui/fixtures/extra/*.md`). The one used here is `~/granit-spike-cave` (a git repo, so `git diff` shows saves). The fixtures already contain a `{#volvo}` anchor in `Wiki targets`, so extra notes must not reuse it.

## Testing

- Manual; this task is the test