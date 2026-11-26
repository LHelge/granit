---
id: zem.hs
title: Backup UI
status: open
priority: P2
created: "2026-11-26T21:09:59.306207Z"
updated: "2026-11-26T21:09:59.306207Z"
tags:
  - gpui
  - port
  - frontend
  - backup
depends_on:
  - zem.4b
---

## Summary

Port the cloud backup UI: sign-in to the backup server, enabling backups for a cave, status, snapshots, restore and snapshot deletion.

## Acceptance Criteria

- [ ] Everything the current backup settings and dialogs offer works in the GPUI app through the core crate's `backup/` module
- [ ] Credentials are handled as today (never logged, stored where the current app stores them)
- [ ] Restore and delete ask for confirmation, as today

## Implementation Notes

- Epic `bb4` ("Backup restore and snapshot deletion") is still open in the current app; port whatever it lands, and check its status before starting.
- Current code: `src-tauri/src/backup/`, backup commands in `src-tauri/src/commands/`, frontend in `src/app/`.

## Testing

- Use the local backup backend dev environment (podman, see the project notes) rather than a real server