---
id: pkx
title: Restore engine and commands
status: open
priority: P1
created: "2026-08-31T11:07:46.918085Z"
updated: "2026-08-31T11:07:46.918085Z"
tags:
  - core
depends_on:
  - "97a"
parent: bb4
---

## Summary
Client-side restore pipeline in `src-tauri/src/backup/` plus tauri commands: download the archive via the presigned GET, verify, decrypt, unpack — into a new directory or over the current cave.

## Acceptance Criteria
- [ ] Download via `BackupApiClient` (add `download_backup(id)` + plain GET of the presigned URL), sha256-verified against the record before decryption
- [ ] Key resolution: use `.granit/backup.key` silently when it exists and its salt matches the archive header; otherwise require a passphrase and derive from the header (needs a small `salt_of(container)`/`decrypt_with_cached` addition in crypto.rs — decrypt(passphrase, …) already exists)
- [ ] New-directory restore: target must be an empty (or nonexistent) directory; unpack, then open it as a cave via the existing open_cave flow
- [ ] In-place rollback: unpack to a temp sibling dir first; on success rename the current cave dir to `<name>.pre-restore-<date>` and move the new dir into place, carrying over `.granit/backup.key`; the old dir is the escape hatch and is left for the user to delete
- [ ] Commands: `restore_backup` (args: backup id, target, optional passphrase, optional credential override — see below) and `list_backups`/`restore` usable with **no cave open** by passing explicit `backend_url`/`api_key` (falls back to the open cave's saved config when omitted)
- [ ] Progress events `restore:progress` (stages Downloading/Decrypting/Unpacking) / `restore:done` / `restore:error`, payload types in granit-types/src/backup.rs
- [ ] Backup and restore are mutually exclusive: share/replace `BackupGuard` with a single operation guard

## Implementation Notes
- Mirror the `backup_now` shape in src-tauri/src/commands/backup.rs (spawn_blocking for decrypt/unpack, no cave mutex across .await)
- `tar::Archive::unpack` already guards path traversal; still validate the target is empty before writing
- After an in-place rollback the frontend must fully refresh — reuse the open_cave path rather than patching state

## Edge Cases
- Wrong passphrase → `BackupError::Decrypt` (already user-readable); sha mismatch → dedicated error; disk-full mid-unpack must not have touched the original cave (temp-dir-then-swap covers this)
- Rollback while a backup is running → guard error

## Testing
- Unit: pack→encrypt→restore round-trip into a temp dir (new-dir path); salt-match vs prompt decision; sha mismatch rejection; in-place swap preserves backup.key and leaves the pre-restore dir