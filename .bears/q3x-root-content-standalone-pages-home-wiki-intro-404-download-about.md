---
id: q3x
title: "Root content + standalone pages: home, wiki intro, 404, download, about"
status: open
priority: P1
created: "2026-06-12T11:38:20.593448310Z"
updated: "2026-06-12T11:38:20.593448310Z"
tags:
  - docs
  - content
depends_on:
  - jfv
parent: sm3
---

## Summary

Final copy for the site's non-wiki surfaces, replacing scaffold placeholders.

## Acceptance Criteria

- [ ] `home.md` — hero copy ("Granit — a minimal, opinionated desktop note-taking app. Your notes live in a local markdown *cave* — with wiki-links, daily notes, todos, and an integrated AI agent.") + a ~6-item feature grid with links into [[getting-started]] and the download page.
- [ ] `wiki.md` — 2–3 sentence wiki index intro.
- [ ] `404.md` — "This note doesn't exist in the cave." + links home/wiki.
- [ ] `pages/download.md` (`order: 1`) — link to GitHub releases/latest, per-OS artifacts (macOS dmg, Linux AppImage/deb/rpm), auto-update note, link to [[installation]].
- [ ] `pages/about.md` (`order: 2`) — philosophy (personal use, no plugins/sync/bloat), tech stack table, dual MIT/Apache-2.0 license, repo link.

## Implementation Notes

- Source: README intro + tech stack table + releases section.
- home.md content renders inside the hero template (`home.html` task in the theme epic) — coordinate on what markup the feature grid uses (plain markdown list vs HTML-in-markdown grid).