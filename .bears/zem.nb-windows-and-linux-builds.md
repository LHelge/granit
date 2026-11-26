---
id: zem.nb
title: Windows and Linux builds
status: open
priority: P2
created: "2026-11-26T21:10:18.548549Z"
updated: "2026-11-26T21:10:18.548549Z"
tags:
  - gpui
  - port
  - release
depends_on:
  - zem.kq
---

## Summary

Build, package and release the GPUI app for Windows and Linux, replacing the current Tauri artifacts and keeping the installation channels (Windows installer, Homebrew, install script) working.

## Acceptance Criteria

- [ ] CI builds Windows and Linux release artifacts on tag
- [ ] Windows: an installer (signed if the current one is), the app starts and opens a cave
- [ ] Linux: a package or AppImage; the app runs on Vulkan (accepted in the spike decisions) on at least one mainstream distribution
- [ ] The installation channels from epic `bym` point at the new artifacts
- [ ] Platform-specific editor behaviour is checked: zorite-editor has Windows keys and IME fixes upstream; the stale-layout fix (`baa.ac`) was only tested on macOS

## Implementation Notes

- GPUI's Linux backend needs the system libraries listed in zorite-editor's CI (`libxkbcommon`, wayland, x11, fontconfig, freetype).
- The updater task decides the per-platform update format; coordinate.

## Testing

- Manual smoke test per platform: open a cave, edit and save a note, stream an agent reply