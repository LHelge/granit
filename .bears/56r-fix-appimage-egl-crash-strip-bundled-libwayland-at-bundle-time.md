---
id: "56r"
title: "Fix AppImage EGL crash: strip bundled libwayland at bundle time"
status: done
priority: P1
created: "2026-06-12T22:00:11.228386414Z"
updated: "2026-06-12T22:08:41.863442125Z"
tags:
  - release
  - ci
  - linux
  - bug
---

## Summary

The released AppImage aborts with `Could not create default EGL display: EGL_BAD_PARAMETER` on rolling-release hosts (reproduced on Arch/Hyprland). Root cause, bisected against the extracted v0.6.0 AppImage: the bundle ships Ubuntu 24.04's `libwayland-{client,cursor,egl,server}`, which the host EGL stack dlopens via the AppImage's LD_LIBRARY_PATH, poisoning EGL display creation even under the GDK_BACKEND=x11 AppRun hook. Removing exactly those four libraries from the AppDir makes the app run.

Tauri's bundler caches `linuxdeploy-plugin-gtk.sh` in `$XDG_CACHE_HOME/tauri/` and only downloads when missing — so the release workflow can pre-seed a patched plugin that `rm`s libwayland from `$APPDIR` at the end, before the appimage output plugin packs and tauri signs. No post-processing or re-signing needed.

## Acceptance Criteria

- [ ] release.yml ubuntu leg pre-seeds the patched plugin before tauri-action
- [ ] A locally built AppImage (isolated XDG_CACHE_HOME) contains no libwayland-*.so and launches on this Arch/Hyprland host
- [ ] Upstream linuxdeploy excludelist behavior noted in the patch comment for future removal