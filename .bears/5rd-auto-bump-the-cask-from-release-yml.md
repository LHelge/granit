---
id: "5rd"
title: Auto-bump the cask from release.yml
status: open
priority: P2
created: "2026-06-12T16:29:55.768747390Z"
updated: "2026-06-12T16:30:16.007277801Z"
tags:
  - release
  - ci
  - macos
depends_on:
  - gfq
parent: bym
---

## Summary

A job appended to `.github/workflows/release.yml` (after `publish`) that updates the tap's cask with the new version and dmg sha256 on every release.

## Acceptance Criteria

- [ ] Job checks out the tap with `TAP_GITHUB_TOKEN`, downloads the published dmg, computes sha256, rewrites version/sha in `Casks/granit.rb`, commits and pushes
- [ ] Failure of this job does not block or fail the release itself (`continue-on-error` or ordering after publish)
- [ ] Verified on the first release after merge (or a re-dispatched release)

## Implementation Notes

- Depends on the tap + `TAP_GITHUB_TOKEN` secret from the cask task.
- Keep it dependency-free shell: gh CLI (preinstalled) to fetch the asset, `sha256sum`, two `sed`-free rewrites (regenerate the full cask file from a heredoc template instead of in-place editing — simpler and deterministic).