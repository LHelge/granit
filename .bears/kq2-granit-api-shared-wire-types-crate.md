---
id: kq2
title: granit-api shared wire-types crate
status: done
priority: P2
created: "2026-08-28T12:45:34.153703Z"
updated: "2026-08-28T12:47:25.163636Z"
parent: psq
---

New wasm-safe workspace member `granit-api/` (serde, chrono, uuid serde-only): CreateBackupRequest/Response, BackupInfo, BackupState, ListBackupsResponse, ApiErrorBody + serde round-trip tests. Add to root Cargo.toml members. Commit: feat(api).