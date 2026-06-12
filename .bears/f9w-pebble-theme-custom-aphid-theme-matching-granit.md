---
id: f9w
title: Pebble theme — custom aphid theme matching Granit
type: epic
status: open
priority: P1
created: "2026-06-12T11:35:35.784135954Z"
updated: "2026-06-12T11:35:35.784135954Z"
tags:
  - docs
  - theme
---

## Scope

A custom aphid theme ("pebble") in `docs/theme/`, bootstrapped from aphid's `default-theme/` and restyled to match Granit: Catppuccin Mocha palette, Inter font, granite pebble logo, docs-style three-column wiki layout (left category nav, prose, right TOC), modern cards, styled code blocks and GitHub alerts. Mocha-only for v1 (no light mode toggle).

Reference: `.claude/skills/aphid-theme/SKILL.md` (installed by the scaffold epic) documents every template variable.

Full plan: /home/lhelge/.claude/plans/i-want-to-create-snazzy-book.md §3

## Acceptance Criteria

- [ ] All 11 templates render via `aphid build` (blog templates remain untouched stubs)
- [ ] Visual identity matches the app: Mocha tokens, Inter, pebble logo in header
- [ ] Wiki pages show left category nav (current page highlighted), TOC sidebar, backlinks card
- [ ] Responsive at ~375px (sidebars collapse)