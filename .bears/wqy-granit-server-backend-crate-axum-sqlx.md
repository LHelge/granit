---
id: wqy
title: granit-server backend crate (axum + sqlx)
status: in_progress
priority: P2
created: "2026-08-28T12:45:39.785918Z"
updated: "2026-08-28T12:47:26.719884Z"
depends_on:
  - kq2
parent: psq
---

backend/ crate: env config, reversible sqlx-cli migrations (api_keys, backups), SHA-256 bearer auth extractor, Storage with dual S3 clients (internal + presign vs S3_PUBLIC_ENDPOINT), routes (create/complete/list), clap CLI (serve/create-key/list-keys/revoke-key), committed .sqlx offline data, #[sqlx::test] handler tests. Commit: feat(server).