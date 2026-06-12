---
id: jz4
title: Add .github/workflows/docs.yml — build with aphid action, deploy to Pages
status: open
priority: P1
created: "2026-06-12T11:36:24.385482021Z"
updated: "2026-06-12T11:36:24.385482021Z"
tags:
  - docs
  - ci
depends_on:
  - jfv
parent: nbd
---

## Summary

GitHub Actions workflow that builds the docs with the official `LHelge/aphid` action and deploys to GitHub Pages, triggered only by docs-relevant pushes to main.

## Acceptance Criteria

- [ ] `.github/workflows/docs.yml` per plan §5: trigger `push` to main with `paths: ['docs/**', '.github/workflows/docs.yml']` + `workflow_dispatch`; permissions `contents: read, pages: write, id-token: write`; concurrency group `pages` (cancel-in-progress false).
- [ ] Build job: checkout → `LHelge/aphid` action (config `docs/aphid.toml`, output `docs/dist`, version `0.3.0`) → configure-pages → upload-pages-artifact.
- [ ] Deploy job: `needs: build`, `github-pages` environment with deployment URL, `actions/deploy-pages`.
- [ ] A push touching only non-docs files does NOT trigger the workflow.

## Implementation Notes

- **Resolve open item:** read `action.yml` in the aphid repo for exact input names/semantics, and prefer a pinned tag ref (e.g. `LHelge/aphid@v0.3.0`) over `@main` if a tag exists.
- Match action versions to what aphid's own docs workflow uses (it deploys aphid.lhelge.se the same way) — living reference: `.github/workflows/` in the aphid repo.

## Testing

- Workflow YAML passes `actionlint` if available, or at minimum a dry parse.
- After merge: workflow run green; deployment visible in the github-pages environment.