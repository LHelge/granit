---
id: gfq
title: Homebrew tap with granit cask
status: open
priority: P1
created: "2026-06-12T16:29:49.741243401Z"
updated: "2026-06-12T16:29:49.741243401Z"
tags:
  - release
  - distribution
  - macos
  - manual
parent: bym
---

## Summary

A personal Homebrew tap (`LHelge/homebrew-tap`) with a cask for the notarized arm64 dmg.

## Manual prerequisites (user)

- [ ] Create the `LHelge/homebrew-tap` GitHub repo (public, empty)
- [ ] Create a fine-grained PAT with contents:write on that repo; add as `TAP_GITHUB_TOKEN` secret in the granit repo (needed by the automation task)

## Acceptance Criteria

- [ ] `Casks/granit.rb` in the tap: current version + sha256, dmg URL from GitHub releases, `depends_on arch: :arm`, `auto_updates true` (the Tauri updater manages updates; brew shouldn't fight it), app stanza, zap stanza clearing app data
- [ ] `brew install --cask lhelge/tap/granit` works on an Apple Silicon Mac
- [ ] Tap README with the one-liner

## Implementation Notes

- dmg is Developer-ID signed and notarized (release.yml uses APPLE_ID/APPLE_TEAM_ID) → no quarantine caveats needed.
- Verify the exact dmg asset naming pattern from the latest release before writing the url stanza.
- The cask file can be authored from this repo and pushed to the tap; user verifies install locally (orchestrator has no macOS).