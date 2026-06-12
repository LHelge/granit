---
id: u8w
title: "theme.css: Catppuccin Mocha tokens, layout, cards, components"
status: done
priority: P1
created: "2026-06-12T11:37:17.266004230Z"
updated: "2026-06-12T12:14:25.912047222Z"
tags:
  - docs
  - theme
  - css
depends_on:
  - qpe
parent: f9w
---

## Summary

Rewrite `docs/theme/static/css/theme.css` as hand-written vanilla CSS (~400–600 lines) carrying the entire visual identity. No Tailwind — an aphid theme is static files with no build step.

## Acceptance Criteria

- [ ] `:root` custom properties named after the app's DaisyUI variables (values from `styles.css:95-123`): `--base-100: #1e1e2e; --base-200: #181825; --base-300: #11111b; --base-content: #cdd6f4; --primary: #cba6f7; --secondary: #b4befe; --accent: #f5e0dc; --neutral: #313244; --info: #89b4fa; --success: #a6e3a1; --warning: #f9e2af; --error: #f38ba8;` plus `--font-sans: "Inter", Avenir, Helvetica, Arial, sans-serif; --radius: 0.75rem;`
- [ ] `color-scheme: dark` (Mocha-only v1, no light mode, no JS toggle).
- [ ] Sticky header on `--base-200` with subtle `--neutral` bottom border + backdrop-blur; content max-width ~72rem; prose line-height 1.7; smooth scroll + `scroll-margin-top` on headings.
- [ ] Cards (wiki index grid, backlinks): `--base-200` surface, 1px `--neutral` border, `--radius` corners, hover lift + `--primary` border tint.
- [ ] Pill tag chips; `--primary` links with `--accent` hover; styled `<details>` collapse for mobile nav/TOC.

## Implementation Notes

- Keep/adapt useful resets from the copied default `theme.css` rather than starting empty.
- Style the three-column wiki grid (left nav / prose / TOC) with CSS grid + breakpoints; single column under ~64rem with sidebars as `<details>`.
- Active TOC highlight: prefer CSS-only (`:target`); skip JS scroll-spy.

## Testing

- Eyeball all page types at desktop/tablet/375px; verify long code lines scroll horizontally, not overflow.