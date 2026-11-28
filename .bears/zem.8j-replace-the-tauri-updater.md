---
id: zem.8j
title: Replace the Tauri updater
status: open
priority: P2
created: "2026-11-26T21:10:18.530658Z"
updated: "2026-11-26T21:10:18.530658Z"
tags:
  - gpui
  - port
  - release
depends_on:
  - zem.kq
---

## Summary

The current app updates itself with `tauri-plugin-updater`, verifying releases with the minisign key whose public half is in `tauri.conf.json`. The GPUI app needs its own update check, download, signature check and install, and existing Tauri installs must be able to update to the first GPUI release.

## Acceptance Criteria

- [ ] The app checks for updates on start (and on demand), shows the release notes, and installs on confirmation
- [ ] Updates are verified with the existing signing key at `~/.tauri/granit.key` / the pubkey in `tauri.conf.json`, or a key change is planned with the owner
- [ ] The last Tauri release can update to the first GPUI release: the update manifest and artifact format it expects are produced for that transition
- [ ] Failure to check or download never blocks using the app

## Implementation Notes

- Read the project notes on the updater signing key and the release workflow quirks before starting.
- Candidates: a small updater on `self_update`, or Sparkle on macOS. Decide per platform with the Windows and Linux task in mind.

## Testing

- End-to-end against a pre-release on a test machine: old version updates to new
## Decisions (owner, 2026-11-28)

- **Own small updater**, not `cargo-packager-updater` (last published July 2025, needs its own manifest format) and not Sparkle. It reads the same `latest.json` as the Tauri updater (one manifest format for old and new apps), verifies with the existing minisign key (cargo-packager signs artifacts in the format Tauri does), and replaces the app per platform: the `.app` bundle on macOS, the AppImage on Linux, the installer on Windows.
- **Silent, as today**: download and install in the background on start; the new version applies on the next launch and shows its release notes then. No confirmation prompt (this replaces "installs on confirmation" in the acceptance criteria).
- The macOS release job (`release-macos`) already adds a signed `Granit_aarch64.app.tar.gz` to `latest.json`; whether an installed Tauri app updates from it is still to be verified end to end.
