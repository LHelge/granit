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

- [ ] G1 Dependencies: `gpui-kit` and `zorite-editor` resolve to one `gpui-pre` version and build on macOS
- [ ] G2 Round-trip: opening and saving every note of a real cave without edits leaves every file byte-identical; a one-character edit produces a one-character diff
- [ ] G3 Wiki-links: `[[note]]`, `[[note|label]]` and heading anchors render, navigate on click, and `[[` offers completion of note slugs and anchors
- [ ] G4 Editing feel: the checklist in the evaluation task has no blocking gaps (Swedish keyboard input and dead keys included)
- [ ] G5 Numbers: startup and idle memory are measurably better than the current release build on the same cave
- [ ] G6 Streaming markdown: an agent-style streamed response renders acceptably in a GPUI Kit markdown view

A failed G2 or G3 triggers the fallback task before a no-go is called.

## Out of scope

Agent chat wiring, settings UI, templates, todos, calendar, tags, search, backup UI, updater, bundling/signing, Windows and Linux builds, extracting a Tauri-free core crate. These belong to the port epic, which is only planned if the spike says go.