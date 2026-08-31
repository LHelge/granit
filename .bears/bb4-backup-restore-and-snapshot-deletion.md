---
id: bb4
title: Backup restore and snapshot deletion
type: epic
status: open
priority: P1
created: "2026-08-31T11:00:15.193705Z"
updated: "2026-08-31T11:00:15.193705Z"
---

## Scope

Complete the cloud backup feature (epic psq, branch `feature/cloud-backup`) with the restore path and snapshot deletion. Decisions agreed with the owner:

- **Restore targets: both** — (a) restore a snapshot into a chosen empty directory and open it as a cave, and (b) roll back the currently open cave in place (destructive, strong confirmation).
- **Entry points** — the Backup settings section of an open cave (saved credentials), and the no-cave state for the lost-machine case with manually entered backend URL + API key. Note: there is no dedicated welcome screen; the no-cave state is the empty reader plus the CaveSelector in the explorer footer — the disaster-restore affordance lives there.
- **Decryption key: cached key when possible** — if `.granit/backup.key` exists and its salt matches the archive header, decrypt silently; otherwise prompt for the passphrase and derive from the header's salt/params (`backup::crypto::decrypt` already implements this).
- **Deletion included** — DELETE endpoint + delete button in the snapshot list; the only way to retire snapshots made under a compromised passphrase.

## Existing groundwork

- `backup::crypto::decrypt(passphrase, container)` and the self-describing archive header (salt + Argon2 params) — src-tauri/src/backup/crypto.rs
- `backups.object_key` column and the `Storage` seam (presign_put/head_size + test stub) — backend/src/storage.rs
- Caddy already routes `/granit-backups/*` for presigned GETs
- `tar::Archive::unpack` (used in tests) protects against path traversal

## Acceptance Criteria

- [ ] A snapshot can be restored to a new directory from settings and from the no-cave state, and opens as a working cave
- [ ] The open cave can be rolled back to a snapshot with the previous contents preserved on disk as an escape hatch
- [ ] Restore works on a machine that has never seen the cave, given only URL + API key + passphrase
- [ ] Snapshots can be deleted (object + record) from the settings list
- [ ] Docs updated (backups.md restore/delete sections, remove the "not implemented yet" note)