---
id: 3ps
title: GitHub Actions CI/CD
type: epic
status: done
priority: P1
created: 2026-03-27T08:34:35.074389Z
updated: 2026-03-27T08:53:20.567791Z
---

## Scope

Set up GitHub Actions for the Granit project:
1. A CI check workflow that runs on PRs to `main` (fmt, clippy, test)
2. A Tauri release workflow triggered by pushing a version tag (`app-v*`)

The release workflow must gate on CI checks passing first.

## Acceptance Criteria

- [ ] PRs to `main` are blocked until fmt, clippy, and tests pass
- [ ] Pushing a tag `v*.*.*` triggers cross-platform Tauri builds (macOS arm64/x86_64, Linux x86_64, Windows x86_64)
- [ ] Release workflow runs CI checks before building
- [ ] Release creates a draft GitHub Release with build artifacts
