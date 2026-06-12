---
id: fge
title: Bootstrap pebble theme from aphid default-theme
status: open
priority: P1
created: "2026-06-12T11:36:31.522984563Z"
updated: "2026-06-12T11:36:31.522984563Z"
tags:
  - docs
  - theme
depends_on:
  - jfv
parent: f9w
---

## Summary

Copy aphid's public `default-theme/` into `docs/theme/`, rename it "pebble", wire it into the config, and confirm an unmodified build. This guarantees all 11 required templates exist before any customization starts.

## Acceptance Criteria

- [ ] `docs/theme/` contains theme.toml + `templates/{404,base,blog_index,blog_post,home,page,pagination,tag,tags_index,wiki_index,wiki_page}.html` + `static/css/theme.css` + `static/js/mermaid.min.js`, copied from https://github.com/LHelge/aphid/tree/main/default-theme.
- [ ] `docs/theme/theme.toml` says `name = "pebble"`, `version = "0.1.0"`.
- [ ] `theme_dir = "theme"` enabled in `docs/aphid.toml`.
- [ ] `aphid build` exits 0 with the copied theme.

## Implementation Notes

- Fetch via raw.githubusercontent.com URLs or a sparse git checkout — do not vendor the whole aphid repo.
- `blog_post.html`, `blog_index.html`, `pagination.html` stay untouched permanently (no blog content → never linked); do not invest time in them in later tasks either.