---
id: zem.kq
title: macOS bundling, signing and notarization
status: open
priority: P1
created: "2026-11-26T21:09:59.322267Z"
updated: "2026-11-26T21:09:59.322267Z"
tags:
  - gpui
  - port
  - release
depends_on:
  - zem.4b
---

## Summary

`cargo tauri build` does the app bundle, icons, signing and notarization today. Replace it for the GPUI app: produce a signed, notarized `.app` and `.dmg` from CI, matching the current release workflow.

## Acceptance Criteria

- [ ] A release build produces `Granit.app` with icon, `Info.plist` (bundle id, version from the workspace, file-access entitlements the app needs) and a `.dmg`
- [ ] The bundle is signed with the existing Developer ID and notarized; Gatekeeper opens it on a clean machine
- [ ] The release GitHub workflow builds it on tag, as today
- [ ] Bundle size and startup recorded against the spike's numbers (30 MB binary, 291 ms warm start)

## Implementation Notes

- Options: `cargo-bundle` or `cargo-packager`, or a short script around `codesign`, `notarytool` and `hdiutil`. Prefer whatever also helps with the Windows and Linux task.
- Keep the bundle id the current app uses, so existing installs update in place, unless the owner decides otherwise.
- The release workflow has known quirks (see the project notes on the release workflow); read them first.

## Testing

- A dry run of the workflow on a pre-release tag