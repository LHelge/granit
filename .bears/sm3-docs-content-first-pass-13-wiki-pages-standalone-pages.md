---
id: sm3
title: Docs content — first pass (~13 wiki pages + standalone pages)
type: epic
status: open
priority: P1
created: "2026-06-12T11:35:41.277168136Z"
updated: "2026-06-12T11:35:41.277168136Z"
tags:
  - docs
  - content
---

## Scope

Write the initial documentation content under `docs/content/`: ~13 wiki pages across four categories (Getting Started, Notes & Writing, AI Agent, Reference), two standalone pages (Download, About), and root files (home.md hero, wiki.md intro, 404.md). Source material: README.md and CLAUDE.md.

Authoring rules (from `.claude/skills/aphid-content/SKILL.md`): page title comes from frontmatter (never a `#` heading in the body); body sections start at `#` (renders as h2); cross-link liberally with `[[slug]]` / `[[slug|label]]` / `[[slug#anchor]]`; use GitHub alerts (`> [!NOTE]` etc.) for callouts. Broken wiki-links fail the build.

Full plan: /home/lhelge/.claude/plans/i-want-to-create-snazzy-book.md §4

## Acceptance Criteria

- [ ] All pages carry correct `category` + `tags` frontmatter
- [ ] `aphid build` passes — zero broken wiki-links
- [ ] Pages cross-link so backlinks render on at least the core pages