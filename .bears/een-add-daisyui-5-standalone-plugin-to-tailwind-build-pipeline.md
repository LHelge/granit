---
id: een
title: Add DaisyUI 5 standalone plugin to Tailwind build pipeline
status: done
priority: P1
created: 2026-04-04T20:02:57.235842782Z
updated: 2026-04-04T20:27:24.089582753Z
tags:
- infra
parent: uqp
---

## Objective
Download DaisyUI 5 standalone plugin files and integrate into the Tailwind CSS 4 build pipeline.

## Steps
1. Download `daisyui.mjs` **and** `daisyui-theme.mjs` from [DaisyUI GitHub releases](https://github.com/saadeghi/daisyui/releases) to project root (next to `styles.css`)
2. Add `@plugin "./daisyui.mjs" { themes: all; }` to `styles.css` (after `@import "tailwindcss"`)
3. Add both `.mjs` files to `.gitignore` (binary vendor files)
4. Document download step in README

## Notes
- `daisyui.mjs` provides the 35 built-in themes
- `daisyui-theme.mjs` is needed for custom theme definitions (Catppuccin flavors)
- No Node.js needed — Tailwind's standalone binary loads `.mjs` plugins directly
- Both files are ~200KB each, from the same release version