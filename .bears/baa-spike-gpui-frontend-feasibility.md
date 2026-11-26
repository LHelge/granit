---
id: baa
title: "Spike: GPUI frontend feasibility"
type: epic
status: open
priority: P1
created: "2026-11-26T12:14:32.646212Z"
updated: "2026-11-26T12:14:32.646212Z"
tags:
  - spike
  - gpui
---

## Scope

Time-boxed spike (target: a few focused sessions) to decide whether Granit's Tauri + Leptos/WASM + CodeMirror frontend should be replaced by a native GPUI frontend with a live-preview (WYSIWYG) markdown editor as the default and only note view.

The spike is a throwaway prototype on branch `spike/gpui`, in a new workspace crate `granit-gpui/`. It reuses the existing backend `Cave` code directly (no IPC). It is **not** the port: nothing here needs to be production quality, and nothing is merged to `main` except the write-up.

Goals the port is meant to achieve (the spike measures these):

- Snappier, more native-feeling UI
- Quicker startup
- Lower memory footprint
- Simpler build (no Node, Tailwind, esbuild, Trunk, wasm target)

## Decisions already made (2026-11-26)

- Mermaid and presentations are dropped from the new frontend; they can be rebuilt later if needed.
- Theming is reduced to a Catppuccin-based light and dark mode.
- The editor never sees frontmatter: `Cave::read_note` returns the body only and `Cave::save_note` re-attaches frontmatter. This stays as is.
- Following fast-moving dependencies (GPUI snapshots, GPUI Kit) is acceptable. Vulkan on Linux is acceptable.
- No separate read-only reader view is planned.

## Stack under test

- `gpui-kit` 0.11 (formerly gpui-component) for chrome: sidebar, tree, lists, theming
- `zorite-editor` 0.11 (MIT) as the live-preview markdown editor; it keeps the markdown source as the buffer and depends on `gpui-pre` ^0.3.12
- GPUI via the `gpui-pre` 0.3.x snapshot family (aliased as `gpui`), because the official `gpui` crate is stale at 0.2.2

## Go / no-go gates

- [x] G1 Dependencies: `gpui-kit` and `zorite-editor` resolve to one `gpui-pre` version and build on macOS
- [ ] G2 Round-trip: opening and saving every note of a real cave without edits leaves every file byte-identical; a one-character edit produces a one-character diff
- [x] G3 Wiki-links: `[[note]]`, `[[note|label]]` and heading anchors render, navigate on click, and `[[` offers completion of note slugs and anchors
- [x] G4 Editing feel: the checklist in the evaluation task has no blocking gaps (Swedish keyboard input and dead keys included)
- [x] G5 Numbers: startup and idle memory are measurably better than the current release build on the same cave
- [x] G6 Streaming markdown: an agent-style streamed response renders acceptably in a GPUI Kit markdown view

A failed G2 or G3 triggers the fallback task before a no-go is called.

## Out of scope

Agent chat wiring, settings UI, templates, todos, calendar, tags, search, backup UI, updater, bundling/signing, Windows and Linux builds, extracting a Tauri-free core crate. These belong to the port epic, which is only planned if the spike says go.
## Handover status (2026-11-26, end of first session)

Branch `spike/gpui`, one commit per task. Resolved stack: gpui-kit **0.10.0** (0.11 pins gpui-pre 0.3.14, which does not compile with zorite-editor 0.11.7 / gpui-bidi 0.1.2), zorite-editor 0.11.7, a single gpui-pre 0.3.12.

| Task | State |
|---|---|
| baa.7f scaffold | done; G1 passes with gpui-kit 0.10 instead of 0.11 |
| baa.85 cave tree | done |
| baa.cs editor + save | done; owner tried it |
| baa.6x round-trip | done; G2 fails as written (see the task's Results) |
| baa.9s wiki-links | done; owner: works fine |
| baa.pb streaming | done; owner: looks awesome |
| baa.ac upstream panic | open; fixed in the owner's fork and sent upstream (issue #4, PR #5 on packetThrower/zorite-editor); the workspace is pinned to the fork branch until a release carries it |
| baa.sp measurements | done for the GPUI side (warm start 291 ms, idle footprint 103 MB, release binary 30 MB; table in the task); Tauri side skipped by the owner, who judged the spike obviously snappier |
| baa.q8 owner evaluation | done; feels great, one gap (fenced code turns into a card at once) |
| baa.jq fallback | cancelled, not needed |
| baa.ng write-up | done; **go** (owner, 2026-11-26); results and draft port items in the task |
| baa.s9 Linux | done; builds and runs on Arch/Hyprland/RADV. Found a quadratic `line_starts` in zorite-editor (large notes crawl), fixed in packetThrower/zorite-editor#6, not in the pinned fork branch; tracked in `zem.vs` |

Running it:

- `cargo run --release -p granit-gpui -- <test-cave>`; use release for judging feel (the 6,900-line note loads in ~1 s in release, ~36 s in debug). `GRANIT_OPEN=<slug>` opens a note at startup.
- `cargo run -p granit-gpui --bin stream_smoke` for the streaming test (G6).
- `granit-gpui --roundtrip <cave> [--save body|editor]` for the G2 check.

Test cave: never the live cave. The fixtures are not in git as a cave; rebuild them in a copy of a small cave with `python3 granit-gpui/fixtures/gen_cave.py <cave>` and copy `granit-gpui/fixtures/extra/*.md` into `<cave>/Spike/Extra/`, then `git init && git add -A && git commit` inside the cave so diffs are easy to read.

Findings not yet in a task body: opening a note with several code blocks froze the app until the host reused one tree-sitter highlighter per language (creating one recompiles the language's queries); fixed in the editor commit.
