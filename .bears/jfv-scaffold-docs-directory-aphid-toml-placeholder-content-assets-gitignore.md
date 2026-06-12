---
id: jfv
title: "Scaffold docs/ directory: aphid.toml, placeholder content, assets, .gitignore"
status: done
priority: P1
created: "2026-06-12T11:36:07.177177556Z"
updated: "2026-06-12T11:56:12.451348493Z"
tags:
  - docs
  - infra
parent: nbd
---

## Summary

Create the `docs/` tree with a working aphid config so `aphid build` passes using the **embedded default theme** (no `theme_dir` yet). This is the foundation every other task builds on.

## Acceptance Criteria

- [ ] `docs/aphid.toml` per plan §2: title "Granit", base_url https://granit.lhelge.se, description, favicon, wiki_default_category "Reference", four wiki categories (Getting Started / Notes & Writing / AI Agent / Reference), `[[socials]]` → github repo. `theme_dir` and `social_image` commented out for now.
- [ ] `docs/content/` with placeholder `home.md`, `404.md`, `wiki.md`, `pages/download.md` (title Download, order 1), `pages/about.md` (title About, order 2), and one stub wiki page (e.g. `wiki/getting-started.md` with `category: Getting Started`).
- [ ] `docs/static/logo.png` (copy of `src-tauri/icons/128x128.png`) and `docs/static/icon.png` (copy of `src-tauri/icons/icon.png`).
- [ ] `.gitignore` gains `/docs/dist/` (existing `/dist/` is root-anchored and does NOT cover it).
- [ ] `cargo install aphid --locked` documented/working locally; `aphid build` from `docs/` exits 0; `aphid serve` renders pages on :3000.

## Implementation Notes

- **Resolve open item:** check aphid's own `docs/aphid.toml` (https://github.com/LHelge/aphid/blob/main/docs/aphid.toml) for the real `wiki_categories` shape — plain name list vs `[[wiki_categories]]` tables with description/icon — and follow it.
- All paths in aphid.toml resolve relative to the file itself.
- Frontmatter rules: standalone pages require `title`; wiki pages take optional `title`/`category`/`tags`.

## Testing

- `cd docs && aphid build` exits 0, output lands in `docs/dist/`, `git status` shows no `dist/` files as untracked.