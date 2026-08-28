---
id: aph
title: Backend CI job
status: done
priority: P2
created: "2026-08-28T12:46:07.857843Z"
updated: "2026-08-28T13:15:09.808369Z"
depends_on:
  - wqy
  - "2ws"
parent: psq
---

Separate ci.yml job with postgres:17 service, SQLX_OFFLINE=true, clippy -p granit-server -p granit-api -D warnings, cargo test -p granit-api -p granit-server; extend .githooks/pre-push with backend clippy. Commit: ci.