---
id: r6f
title: README "Documentation" section (local docs workflow)
status: open
priority: P3
created: "2026-06-12T11:38:44.921223943Z"
updated: "2026-06-12T11:38:44.921223943Z"
tags:
  - docs
  - polish
depends_on:
  - jfv
parent: hb5
---

## Summary

Document the docs-site workflow in the root README so contributors (and future you) know how to preview and extend the site.

## Acceptance Criteria

- [ ] README.md gains a short "Documentation" section: link to https://granit.lhelge.se, `cargo install aphid --locked`, `cd docs && aphid serve` / `aphid build`, scaffolding via `aphid wiki new` / `aphid page new`, note that the Docs workflow deploys on docs-path pushes to main.
- [ ] No `docs/README.md` — keep non-content markdown out of the docs tree.