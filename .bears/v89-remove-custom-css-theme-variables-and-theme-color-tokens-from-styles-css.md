---
id: v89
title: Remove custom CSS theme variables and @theme color tokens from styles.css
status: done
priority: P1
created: 2026-04-04T20:03:50.143247013Z
updated: 2026-04-04T20:36:18.667641346Z
tags:
- css
depends_on:
- een
parent: uqp
---

## Summary

Replace the custom `--theme-*` CSS variables and `@theme` color token mappings in `styles.css` with daisyUI's semantic color system. Remove the `:root` fallback colors.

## Implementation Notes

### Remove from `styles.css`:
1. **Remove the entire `@theme { }` block** (lines 4-28) that maps `--color-window`, `--color-panel`, etc. to `--theme-*` CSS variables
2. **Remove the `:root { }` block** (lines 31-51) with default `--theme-*` hex values

### Keep in `styles.css`:
1. `@import "tailwindcss";`
2. `@plugin "@tailwindcss/typography";`
3. `@plugin "./daisyui.mjs" { themes: all; }` (from step 1)
4. Prose typography overrides — but update them:
   - Replace `var(--theme-accent)` → daisyUI's `oklch(var(--p))` or just use `primary` color variable
   - Replace `var(--theme-fg-faint)` → daisyUI's `base-content` with opacity
5. `.broken-link` styles — update to use daisyUI color variables
6. `.titlebar` and toast animation — keep as-is

### Color mapping reference (for prose/broken-link CSS):
| Old CSS variable | DaisyUI equivalent |
|---|---|
| `var(--theme-accent)` | `oklch(var(--color-primary))` or Tailwind `primary` |
| `var(--theme-fg-faint)` | `oklch(var(--color-base-content) / 0.4)` |
| `var(--theme-fg-secondary)` | `oklch(var(--color-base-content) / 0.7)` |

### Font override:
Keep the `--font-sans` override in a `@theme` block if needed:
```css
@theme {
  --font-sans: Inter, Avenir, Helvetica, Arial, sans-serif;
}
```

## Files to Modify
- `styles.css` — remove custom theme vars, update prose/broken-link to use daisyUI colors

## Acceptance Criteria
- [ ] No `--theme-*` CSS variables remain
- [ ] No `--color-window`, `--color-panel`, etc. custom color tokens remain
- [ ] Prose link styling uses daisyUI primary color
- [ ] Broken-link styling uses daisyUI base-content colors
- [ ] Tailwind build succeeds