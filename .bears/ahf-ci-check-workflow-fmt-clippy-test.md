---
id: ahf
title: CI check workflow (fmt, clippy, test)
status: done
priority: P1
created: 2026-03-27T08:34:52.515585Z
updated: 2026-03-27T08:53:16.246193Z
tags:
- ci
parent: 3ps
---

## Summary

Create `.github/workflows/ci.yml` — a workflow that runs on pull requests targeting `main`. It checks formatting (`cargo fmt`), linting (`cargo clippy`), and runs all tests (`cargo test`) for both workspace crates.

## Acceptance Criteria

- [ ] Workflow triggers on `pull_request` to `main` and `workflow_dispatch`
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes (both native and wasm targets)
- [ ] `cargo test -p granit` passes (backend tests)
- [ ] Rust toolchain and dependencies are cached via `swatinem/rust-cache`
- [ ] Linux system deps (libwebkit2gtk, etc.) are installed for the Tauri backend build
- [ ] `trunk` and `tailwindcss` CLI are installed for frontend checks

## Implementation Notes

- File: `.github/workflows/ci.yml`
- Runner: `ubuntu-22.04` (single platform is sufficient for checks)
- This is a Rust workspace with two crates:
  - Root `granit-ui` (Leptos/WASM frontend) — needs `wasm32-unknown-unknown` target
  - `src-tauri/` `granit` (Tauri backend) — needs Linux system deps
- Steps:
  1. `actions/checkout@v4`
  2. Install Linux system deps (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`)
  3. `dtolnay/rust-toolchain@stable` with `targets: wasm32-unknown-unknown` and `components: rustfmt, clippy`
  4. `swatinem/rust-cache@v2` with `workspaces: './src-tauri -> target'`
  5. Install `trunk` CLI (`cargo install trunk` or binary download)
  6. Install `tailwindcss` standalone CLI (download binary from GitHub releases)
  7. `cargo fmt --all -- --check`
  8. `cargo clippy --workspace --all-targets -- -D warnings`
  9. `cargo test -p granit`

## Edge Cases

- The WASM crate can't run native tests — only run `cargo test -p granit`
- `trunk` is needed so clippy can resolve the frontend build properly
- `tailwindcss` must be on PATH for the Trunk pre-build hook

## Testing

- Push a PR to `main` and verify all three checks pass
- Intentionally break formatting and confirm the workflow fails
