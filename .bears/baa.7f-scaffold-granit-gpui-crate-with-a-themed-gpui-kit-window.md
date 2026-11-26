---
id: baa.7f
title: Scaffold granit-gpui crate with a themed GPUI Kit window
status: done
priority: P1
created: "2026-11-26T12:14:43.000271Z"
updated: "2026-11-26T12:50:58.730652Z"
tags:
  - spike
  - gpui
attempts: 1
---

## Summary

Create branch `spike/gpui` and a new workspace binary crate `granit-gpui/` that opens an empty GPUI Kit window with a Catppuccin light/dark theme. This settles gate G1: whether the dependency set resolves and builds at all.

## Acceptance Criteria

- [ ] Branch `spike/gpui` exists; `granit-gpui` is a member of the root workspace in `Cargo.toml`
- [ ] `Cargo.toml` depends on `gpui = { package = "gpui-pre", version = "0.3" }`, `gpui-kit = "0.11"` and `zorite-editor = "0.11"`, and `cargo tree -p granit-gpui -i gpui-pre` shows exactly one `gpui-pre` version
- [ ] `cargo run -p granit-gpui` opens a window on macOS with a sidebar placeholder and a main area
- [ ] Two themes exist, Catppuccin Latte (light) and Catppuccin Frappé (dark), and follow the OS appearance
- [ ] Clean build time and incremental build time (touch `main.rs`, rebuild) are written down for the measurements task

## Implementation Notes

- Colors: copy the Catppuccin Latte and Frappé values from `styles.css` (the two `@plugin "daisyui/theme"` blocks) into GPUI Kit theme definitions. GPUI Kit has a theme schema (`.theme-schema.json` in its repo); check its docs at gpui-kit.com for the current API, it changes weekly.
- `gpui-pre` snapshots are each a different Zed commit: every GPUI-facing crate must agree on one version. If `gpui-kit` 0.11 and `zorite-editor` 0.11 cannot unify, try adjacent versions of either before declaring G1 failed, and record what worked.
- Commit `Cargo.lock` changes on the spike branch only.
- Do not touch `src/`, `js/` or the Tauri config; the existing app must keep building.

## Edge Cases

- `cargo clippy -p granit-ui --target wasm32-unknown-unknown` must not start picking up the new crate. The new crate is native-only; if workspace-wide commands break, exclude it via `default-members` rather than removing it from the workspace.

## Testing

- `cargo build -p granit-gpui` and `cargo fmt --all` pass
- Existing `cargo test -p granit` still passes