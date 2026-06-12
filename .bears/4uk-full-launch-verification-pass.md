---
id: "4uk"
title: Full launch verification pass
status: open
priority: P1
created: "2026-06-12T11:39:07.355238467Z"
updated: "2026-06-12T11:39:07.355238467Z"
tags:
  - docs
  - verify
depends_on:
  - "92s"
  - dh5
parent: hb5
---

## Summary

End-to-end verification of the live site against the plan's checklist (§7).

## Acceptance Criteria

- [ ] `aphid build` exits 0 locally — zero broken wiki-links
- [ ] Eyeball via `aphid serve`: home hero, 4-category wiki index grid, wiki pages (left nav highlights current page, TOC anchors scroll, backlinks card on a linked-to page), code block in Mocha `hl-*` colors, every alert type, tags index, themed 404, header nav order (Wiki, Download, About), favicon in tab
- [ ] Responsive at ~375px: sidebars collapse to `<details>`, no horizontal scroll
- [ ] Push a non-docs commit to main → Docs workflow does NOT run (paths filter works)
- [ ] Live site: https://granit.lhelge.se with HTTPS; `/wiki/`, deep wiki URL, `/download/`, bogus URL → themed 404
- [ ] Page source: canonical URLs and og/social meta point at https://granit.lhelge.se/...