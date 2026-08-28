---
id: psq
title: Cloud backup for caves
type: epic
status: open
priority: P2
created: "2026-08-28T12:45:28.845557Z"
updated: "2026-08-28T12:45:28.845557Z"
---

Self-hosted cave backup: Granit packs the cave into a tar+zstd archive encrypted with a passphrase-derived key (Argon2id, salt+params in the archive header), uploads it via a presigned S3 PUT URL to RustFS, and a new axum+sqlx backend keeps the snapshot catalog in postgres. 4 docker containers (postgres, RustFS, granit-server, Caddy). Shared wire-types crate `granit-api`. Scope: take + upload + list; download/restore is a future epic.

Full plan: ~/.claude/plans/i-m-thinking-of-adding-lexical-lantern.md