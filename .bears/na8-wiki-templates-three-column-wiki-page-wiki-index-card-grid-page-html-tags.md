---
id: na8
title: "Wiki templates: three-column wiki_page, wiki_index card grid, page.html, tags"
status: done
priority: P1
created: "2026-06-12T11:37:02.103906311Z"
updated: "2026-06-12T12:17:33.780556339Z"
tags:
  - docs
  - theme
depends_on:
  - fge
parent: f9w
---

## Summary

The core docs reading experience: a three-column wiki page layout with persistent category navigation, plus the wiki index, standalone page, and tag templates.

## Acceptance Criteria

- [ ] `wiki_page.html` wide layout: **left sidebar** = all wiki pages grouped by category from the `wiki_categories` variable, current page highlighted (`page.url == url`); **center** = prose column ~70ch with category badge + tag chips above the `<h1>` title; **right** = sticky TOC from `toc[]` (level/id/text). Backlinks card ("Linked from") from `backlinks[]` below the content, only when non-empty.
- [ ] Mobile (~375px): single column; left nav and TOC collapse into `<details>` elements above the content.
- [ ] `wiki_index.html`: intro from `wiki.md` content, then a rounded-card grid from `categories` (category name + page links per card).
- [ ] `page.html`: centered prose column (title + content).
- [ ] `tag.html` / `tags_index.html`: light restyle with Mocha tag chips (wiki pages carry tags, so these render real pages).

## Implementation Notes

- All variable shapes in `.claude/skills/aphid-theme/SKILL.md`: `wiki_categories` entries have `name` (string or null) and `pages` `{title, url}`; uncategorized appear under `name = null` — render those last under the `wiki_default_category` label or skip if empty.
- `created`/`updated`/`tags` are optional on wiki pages — guard with `{% if %}`.
- Heading anchors: add `scroll-margin-top` styling hooks (CSS task) and make TOC links target `#{{ entry.id }}`.

## Testing

- Serve and check: a page with TOC + backlinks, a page with neither, the index grid with all four categories, a tag page.