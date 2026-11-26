---
id: bgy.7j
title: "docs: backup pages, one source for the server setup"
status: open
priority: P2
created: "2026-11-26T23:42:04.957195177Z"
updated: "2026-11-26T23:42:04.957195177Z"
tags:
  - docs
---

## Summary

Fix `docs/content/wiki/backups.md` and `backup-server.md`. Make `backup-server.md` the single user-facing source for running the server, and trim `backend/README.md` to the quick start, development and test commands, and a link to it.

## Acceptance Criteria

- [ ] **backups:**
  - "The token created above" points to the token in backup-server.
  - There is no passphrase prompt: the passphrase is set once in Settings → Backup before the first backup (`commands/backup.rs` returns `PassphraseNotSet`).
  - Say that each archive is limited to 1 GiB.
  - The two passages on passphrase changes are merged into one.
  - The server setup is one sentence plus a link.
  - The Argon2 bounds and Unix-permission details are cut.
- [ ] **backups, Replace current cave:** say what happens to `.git` today. It is excluded from the archive, so after an in-place restore it exists only in `<cave>.pre-restore-*`. If the fix in the restore bug task has landed, describe the fixed behaviour instead.
- [ ] **backup-server, copying files:** "Copy just those two files" also says to replace `build:` with `image:`, since building needs the whole repository.
- [ ] **backup-server, security:** a leaked token can also download ciphertext.
- [ ] **backup-server, versions:** version examples use `<version>`.
- [ ] **backup-server, moved from `backend/README.md`:**
  - the 1 GiB limit
  - the upload-integrity requirements (S3 checksums and `If-None-Match` conditional PUTs, for anyone replacing RustFS)
  - the Traefik / reverse-proxy setup
  - the env-variable defaults
- [ ] **`backend/README.md`:** keeps the quick start, development and test commands, and links to the docs page for the rest. The two disagree on nothing.

## Implementation Notes

Load the `aphid-content` skill. Facts were verified against `backend/src/` and `src-tauri/src/backup/`. Re-check the limits and routes before moving them.