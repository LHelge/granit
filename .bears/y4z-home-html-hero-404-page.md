---
id: y4z
title: home.html hero + 404 page
status: done
priority: P1
created: "2026-06-12T11:36:53.049485850Z"
updated: "2026-06-12T12:16:14.002609447Z"
tags:
  - docs
  - theme
depends_on:
  - fge
parent: f9w
---

## Summary

Landing page with a hero section and a themed 404.

## Acceptance Criteria

- [ ] `home.html`: large pebble logo, tagline, two CTA buttons ("Download" → `/download/`, "Read the docs" → `/wiki/`), then `{{ home.content | safe }}` for the markdown feature grid from `home.md`. Recent-blog-posts block removed.
- [ ] `404.html`: pebble logo, renders `not_found.content` when `content/404.md` exists, links home + wiki index.

## Implementation Notes

- Variable shapes per `.claude/skills/aphid-theme/SKILL.md`: home.html gets `home` (object?, `.content`) and `posts`; 404.html gets `not_found` (object?). Guard both with `{% if %}`.
- Hero copy itself lives in `home.md` / `404.md` (content epic) — this task is layout only; keep placeholder copy rendering correctly.