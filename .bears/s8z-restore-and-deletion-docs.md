---
id: s8z
title: Restore and deletion docs
status: open
priority: P3
created: "2026-08-31T11:08:14.321329Z"
updated: "2026-08-31T11:08:14.321329Z"
tags:
  - docs
depends_on:
  - yxj
  - "7rt"
  - "3ph"
parent: bb4
---

## Summary
Update docs/content/wiki/backups.md: remove the "restoring is not implemented yet" note, add sections for restoring (both targets, the disaster flow, cached-key vs passphrase behavior, the `.pre-restore` escape-hatch directory) and for deleting snapshots (including that deletion is how snapshots under a retired passphrase are actually removed).

## Acceptance Criteria
- [ ] backups.md updated; intro no longer says restore is pending
- [ ] `aphid build` passes in docs/ (validates wiki-links)

## Implementation Notes
- Follow the aphid-content skill conventions (`#` = h2, wiki-links, GitHub-style alerts)