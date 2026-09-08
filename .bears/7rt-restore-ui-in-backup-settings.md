---
id: "7rt"
title: Restore UI in Backup settings
status: in_progress
priority: P1
created: "2026-08-31T11:07:56.891306Z"
updated: "2026-09-08T13:52:14.176168Z"
tags:
  - ui
depends_on:
  - pkx
parent: bb4
---

## Summary
Per-snapshot restore actions in the settings snapshot list (src/app/settings/backup.rs): restore to a new folder, or roll back the current cave.

## Acceptance Criteria
- [ ] Each complete snapshot row gets a restore action offering the two targets; pending rows get none
- [ ] New-folder path: folder picker (`ipc::pick_folder`), then restore with progress stages shown (reuse the spinner+stage pattern from Back up now)
- [ ] Roll-back path: explicit confirmation spelling out that current contents are replaced and where the pre-restore copy is left
- [ ] Passphrase prompt appears only when the engine reports the cached key is unusable (salt mismatch / missing key file)
- [ ] `restore:done` closes the flow and the app switches to the restored cave; errors land in the section like backup errors

## Implementation Notes
- ipc wrappers + `listen_restore_*` mirroring the `listen_backup_*` trio
- Remember the DaisyUI `.label` nowrap trap for any new help text
- Keep the section within the ~440px content pane — no side-by-side rows that can't wrap