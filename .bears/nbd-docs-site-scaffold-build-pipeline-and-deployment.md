---
id: nbd
title: "Docs site: scaffold, build pipeline, and deployment"
type: epic
status: open
priority: P1
created: "2026-06-12T11:35:31.163332523Z"
updated: "2026-06-12T11:35:31.163332523Z"
tags:
  - docs
  - infra
---

## Scope

Stand up the documentation site infrastructure: a `/docs` directory built with aphid (v0.3.0), the aphid Claude Code skills installed repo-locally, a GitHub Actions workflow deploying to GitHub Pages, and the custom domain granit.lhelge.se live.

Full plan: /home/lhelge/.claude/plans/i-want-to-create-snazzy-book.md

## Acceptance Criteria

- [ ] `aphid build` from `docs/` exits 0
- [ ] Docs workflow runs on docs-path pushes to main only, and deploys to Pages
- [ ] https://granit.lhelge.se serves the site with valid HTTPS