---
id: zem
title: Port the frontend to GPUI
type: epic
status: open
priority: P1
created: "2026-11-26T21:08:43.315072Z"
updated: "2026-11-26T21:08:43.315072Z"
tags:
  - gpui
  - port
---

## Scope

Replace the Tauri + Leptos/WASM + CodeMirror frontend with a native GPUI app: GPUI Kit for the chrome, `zorite-editor` as the live-preview note editor (the only note view), and GPUI Kit's `TextView` for agent chat. The backend logic stays, moved into a Tauri-free core crate that the GPUI app calls directly (no IPC).

The spike (epic `baa`, branch `spike/gpui`) called a go on 2026-11-26. Its write-up, gate results and evidence are in the body of `baa.ng`; read it before starting any task here. The port starts from a clean branch off `main` and reuses spike code selectively; `spike/gpui` is never merged.

## Decisions carried over from the spike

- Mermaid and presentations are dropped from the new frontend for now (see the decision task in this epic).
- Theming is Catppuccin light and dark only.
- No separate read-only reader view.
- The editor never sees frontmatter: `Cave::read_note` returns the body and `Cave::save_note` re-attaches frontmatter.
- Following fast-moving dependencies (gpui-pre snapshots, GPUI Kit) is acceptable. Vulkan on Linux is acceptable.

## Stack

- `gpui-pre` (aliased as `gpui`), one version shared by every GPUI crate. The spike used 0.3.12.
- `gpui-kit` 0.10 until `zorite-editor` builds against the gpui-pre that gpui-kit 0.11+ pins.
- `zorite-editor` (MIT), pinned to `LHelge/zorite-editor` branch `fix/stale-layout-input` until upstream PR packetThrower/zorite-editor#5 is released (tracked in `baa.ac`).

## Acceptance Criteria

- [ ] The GPUI app covers everything the current app does, except the features dropped above
- [ ] The release build, updater, signing and notarization work on macOS; Windows and Linux builds exist
- [ ] The Tauri/Leptos frontend, Node, Tailwind, esbuild, Trunk and the wasm target are gone from the repo, CI and git hooks
- [ ] Saving a note without edits leaves the file byte-identical (spike gate G2)
- [ ] The docs site describes the new app

## Out of scope

Sync, plugins, file watching, a backlinks panel: still deferred as in `CLAUDE.md`.