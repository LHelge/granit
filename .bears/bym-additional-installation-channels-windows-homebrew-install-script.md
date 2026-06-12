---
id: bym
title: "Additional installation channels: Windows, Homebrew, install script"
type: epic
status: open
priority: P2
created: "2026-06-12T16:29:26.762321443Z"
updated: "2026-06-12T16:29:26.762321443Z"
tags:
  - release
  - distribution
---

## Scope

Broaden how Granit can be installed: a Windows release target (NSIS/MSI via the existing tauri-action matrix), a personal Homebrew tap with an auto-bumped cask for the notarized macOS dmg, a `curl | sh` install script for Linux served from the docs site, and proper build-from-source documentation. Explicitly out of scope: publishing to crates.io for `cargo install` (Tauri GUI app — frontend asset pipeline makes it impractical) and Windows code signing (ship unsigned, document SmartScreen).

## Acceptance Criteria

- [ ] Tagged releases produce Windows installers and the updater manifest gains a windows-x86_64 entry
- [ ] `brew install --cask lhelge/tap/granit` installs Granit; cask bumps automatically on release
- [ ] `curl -fsSL https://granit.lhelge.se/static/install.sh | sh` installs the AppImage with desktop integration on x86_64 Linux
- [ ] installation/download docs cover all channels including build-from-source