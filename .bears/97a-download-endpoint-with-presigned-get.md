---
id: "97a"
title: Download endpoint with presigned GET
status: in_progress
priority: P1
created: "2026-08-31T11:00:24.468065Z"
updated: "2026-08-31T11:12:01.625764Z"
tags:
  - server
  - api
parent: bb4
---

## Summary
Server side of restore: `GET /api/v1/backups/{id}/download` returns a time-limited presigned S3 GET URL for a complete snapshot.

## Acceptance Criteria
- [ ] `DownloadBackupResponse { download_url, download_expires_at }` added to granit-api with serde round-trip test
- [ ] Route returns 404 for unknown/foreign ids, 409 (Conflict) for `pending` snapshots
- [ ] Presigned URL is signed against `S3_PUBLIC_ENDPOINT` (the `presign` client), 15 min TTL like uploads

## Implementation Notes
- `Storage::presign_get(key, expires)` next to `presign_put` in backend/src/storage.rs; extend the `#[cfg(test)]` stub the same way
- Handler in backend/src/routes/backups.rs, scoped by `AuthedKey` like the others; route registered in routes/mod.rs
- After query changes: `cd backend && cargo sqlx prepare -- --all-targets` and commit `.sqlx`

## Testing
- `#[sqlx::test]` handler tests: happy path (stub URL contains object_key), pending → 409, foreign key's backup → 404