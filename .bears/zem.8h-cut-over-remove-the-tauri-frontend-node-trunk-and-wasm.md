---
id: zem.8h
title: "Cut over: remove the Tauri frontend, Node, Trunk and wasm"
status: open
priority: P1
created: "2026-11-26T21:10:30.012817Z"
updated: "2026-11-27T01:51:11.310571141Z"
tags:
  - gpui
  - port
  - infra
depends_on:
  - zem.4k
  - zem.up
  - zem.2u
  - zem.5c
  - zem.nj
  - zem.hs
  - zem.8j
  - zem.kq
  - zem.t7
  - zem.nb
  - zem.33
  - zem.ru
  - zem.cn
  - zem.dz
---

## Summary

Once the GPUI app has feature parity, delete the old frontend and its toolchain so the build is cargo-only.

## Acceptance Criteria

- [ ] Removed: the `granit-ui` crate (`src/`), `index.html`, Trunk config, `js/`, `package.json`, `package-lock.json`, Tailwind/DaisyUI and esbuild config, `build/`
- [ ] `src-tauri/` reduced to nothing that is still used, or removed, along with `tauri.conf.json` once the updater no longer needs it
- [ ] CI runs fmt, clippy and tests for the remaining crates only; no `npm`, no wasm target, no wasm-pack
- [ ] The `.githooks/` `pre-commit` and `pre-push` hooks match the new CI
- [ ] `CLAUDE.md` describes the new layout, build and architecture
- [ ] A clean clone builds and runs with `cargo` alone

## Implementation Notes

- Do this as the last feature step, in one focused commit per concern if it gets large; release notes are generated from commit messages.
- Keep `granit-types` only if something still needs it after the IPC layer is gone.

## Testing

- CI green on the cut-over branch; a release dry run