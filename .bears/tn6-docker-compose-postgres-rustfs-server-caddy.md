---
id: tn6
title: "Docker compose: postgres + RustFS + server + Caddy"
status: open
priority: P2
created: "2026-08-28T12:45:45.736336Z"
updated: "2026-08-28T12:45:45.736336Z"
depends_on:
  - wqy
parent: psq
---

Multi-stage Dockerfile (repo-root context, SQLX_OFFLINE), docker-compose.yml (postgres:17-alpine, rustfs, backend, caddy:2-alpine — only Caddy host-published), Caddyfile routing /api/* → backend and /granit-backups/* → rustfs, root .dockerignore, backend/README.md documenting S3_PUBLIC_ENDPOINT = Caddy origin. Commit: feat(server).