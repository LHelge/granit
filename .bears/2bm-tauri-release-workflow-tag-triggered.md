---
id: 2bm
title: Tauri release workflow (tag-triggered)
status: done
priority: P1
created: 2026-03-27T08:35:01.374509Z
updated: 2026-03-27T08:53:20.567593Z
tags:
- ci
depends_on:
- ahf
parent: 3ps
---

## Summary

Create `.github/workflows/release.yml` — a workflow triggered by pushing a tag matching `v*.*.*`. It first runs the CI checks (reusing the CI workflow), then builds the Tauri app for all target platforms and creates a draft GitHub Release with the artifacts.

## Acceptance Criteria

- [ ] Workflow triggers on `push.tags: ['v*.*.*']` and `workflow_dispatch`
- [ ] CI checks (fmt, clippy, test) run first and gate the release build
- [ ] Builds for: macOS arm64, macOS x86_64, Linux x86_64, Windows x86_64
- [ ] Uses `tauri-apps/tauri-action@v0` for building and releasing
- [ ] Creates a draft GitHub Release with tag `v__VERSION__`
- [ ] Build artifacts are attached to the release
- [ ] Rust toolchain and deps are cached per platform

## Implementation Notes

- File: `.github/workflows/release.yml`
- **Gate on CI**: Use a two-job workflow:
  1. `check` job: reuse CI workflow via `uses: ./.github/workflows/ci.yml` (workflow_call) or duplicate the check steps
  2. `build` job: `needs: [check]` — only runs if checks pass
- Alternatively, add `workflow_call` trigger to `ci.yml` so the release workflow can call it directly
- **Build matrix** (from Tauri docs):
  - `macos-latest` with `--target aarch64-apple-darwin`
  - `macos-latest` with `--target x86_64-apple-darwin`
  - `ubuntu-22.04` (x86_64)
  - `windows-latest` (x86_64)
- Steps per platform:
  1. `actions/checkout@v4`
  2. Install Linux system deps (ubuntu only)
  3. Install `trunk` CLI and `tailwindcss` CLI (needed for `beforeBuildCommand`)
  4. `dtolnay/rust-toolchain@stable` with macOS cross-compile targets
  5. `swatinem/rust-cache@v2`
  6. `tauri-apps/tauri-action@v0` with:
     - `tagName: v__VERSION__`
     - `releaseName: 'Granit v__VERSION__'`
     - `releaseDraft: true`
     - `prerelease: false`
     - `args: ${{ matrix.args }}`
- No Node.js setup needed — this project uses Trunk (Rust/WASM), not npm
- `trunk` must be installed on all platforms (it's the `beforeBuildCommand`)
- `tailwindcss` standalone binary differs per OS — install the right one

## Edge Cases

- `GITHUB_TOKEN` needs write permission for creating releases (Settings → Actions → Workflow permissions)
- macOS runners need both `aarch64-apple-darwin` and `x86_64-apple-darwin` Rust targets
- `tailwindcss` binary name/URL differs per platform (linux-x64, macos-arm64, windows-x64, etc.)
- Ensure `trunk` is available on Windows (may need `cargo-binstall` or `cargo install`)

## Release Process

1. Bump version in `src-tauri/tauri.conf.json` and root `Cargo.toml`
2. Commit: `chore: bump version to 0.x.0`
3. Tag: `git tag v0.x.0`
4. Push: `git push && git push --tags`
5. Workflow triggers → checks → builds → draft release created
6. Review draft release on GitHub → publish when ready

## Testing

- Push a test tag and verify builds trigger after checks pass
- Verify artifacts appear on the draft release for all platforms
