---
id: q4z
title: Client packing + passphrase crypto
status: in_progress
priority: P2
created: "2026-08-28T12:45:51.769685Z"
updated: "2026-08-28T13:09:04.411528Z"
parent: psq
---

src-tauri/src/backup/{mod,archive,crypto}.rs: recursive walk → tar → zstd (excludes backup.key, embeddings.bin, .git), GRNT container format with Argon2id params + salt + XChaCha20 nonce in header, key cache .granit/backup.key, BackupError. Tests: round-trip, exclusions, wrong passphrase, tampered ciphertext, corrupt cache. Commit: feat(backup).