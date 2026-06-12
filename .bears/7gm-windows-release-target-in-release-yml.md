---
id: "7gm"
title: Windows release target in release.yml
status: open
priority: P1
created: "2026-06-12T16:29:42.400011964Z"
updated: "2026-06-12T16:29:42.400011964Z"
tags:
  - release
  - ci
  - windows
parent: bym
---

## Summary

Add `windows-latest` to the release build matrix so tagged releases ship NSIS (.exe) and MSI installers with updater artifacts.

## Acceptance Criteria

- [ ] Matrix entry `platform: windows-latest` (x86_64, empty args) in `.github/workflows/release.yml`; Apple cert steps stay macOS-gated (they already are via `if:`)
- [ ] A `workflow_dispatch` or test-tag run produces .exe/.msi bundles and updater artifacts; `latest.json` gains a windows-x86_64 entry
- [ ] In-app updater config needs no change (minisign keys are platform-independent)

## Implementation Notes

- `tauri.conf.json` already has `targets: "all"` + `createUpdaterArtifacts: true` — no config change expected.
- Compile risk to verify on the runner: `fastembed`/onnxruntime (ort) on Windows. If ort download/link fails, investigate ort's prebuilt Windows binaries feature flags before anything else.
- Trunk/npm/wasm32 steps mirror the existing matrix rows; mind bash-vs-pwsh in any `run:` steps added (existing changelog steps are Linux-job only, untouched).
- Unsigned binaries → SmartScreen warning; documented in the docs task, not solved here.

## Testing

- Dry-run the build via workflow_dispatch (release stays draft until publish job; check artifacts on the draft, then delete the draft if it was only a test).