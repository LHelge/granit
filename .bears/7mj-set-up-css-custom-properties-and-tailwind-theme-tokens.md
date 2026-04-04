---
id: 7mj
title: Set up CSS custom properties and Tailwind theme tokens
status: done
priority: P1
created: 2026-04-02T23:10:57.955801991Z
updated: 2026-04-03T23:51:11.473892214Z
tags:
- frontend
depends_on:
- pba
parent: njk
---

## Summary
Define CSS custom properties for all theme colors in `styles.css` and update Tailwind's `@theme` block to reference them. This creates the semantic color tokens the UI will use.

## Acceptance Criteria
- [ ] CSS custom properties defined for all 26 palette colors (`--color-base`, `--color-mantle`, `--color-crust`, `--color-surface0`..`2`, `--color-overlay0`..`2`, `--color-subtext0`..`1`, `--color-text`, `--color-rosewater`, `--color-flamingo`, `--color-pink`, `--color-mauve`, `--color-red`, `--color-maroon`, `--color-peach`, `--color-yellow`, `--color-green`, `--color-teal`, `--color-sky`, `--color-sapphire`, `--color-blue`, `--color-lavender`)
- [ ] Tailwind `@theme` block maps these to utility classes (e.g. `--color-base` → `bg-base`, `text-base`)
- [ ] Default values set to Mocha so the app works without JS theme injection
- [ ] Document the mapping from Catppuccin style guide roles → CSS classes

## Implementation Notes
- Update `styles.css` `@theme` block
- Use `:root` for CSS variables, updated dynamically by frontend JS
- Consider semantic aliases if needed (e.g. `--color-error: var(--color-red)`)
- Tailwind v4 uses `@theme` for custom colors — define them there