---
id: zh9
title: Define 4 Catppuccin flavors as custom DaisyUI themes
status: done
priority: P1
created: 2026-04-04T20:23:33.126299881Z
updated: 2026-04-04T20:30:59.992686813Z
tags:
- css
- theme
depends_on:
- een
---

## Objective
Define all 4 Catppuccin flavors as custom DaisyUI themes in `styles.css`.

## Catppuccin → DaisyUI Color Mapping

Each flavor uses the same semantic mapping:

| DaisyUI Variable | Catppuccin Role | Purpose |
|---|---|---|
| `color-scheme` | light (Latte) / dark (others) | Browser UI hint |
| `--color-base-100` | Base | Main background |
| `--color-base-200` | Mantle | Secondary background |
| `--color-base-300` | Crust | Tertiary background |
| `--color-base-content` | Text | Default text |
| `--color-primary` | Mauve | Primary accent |
| `--color-primary-content` | Crust (dark) / Base (light) | Text on primary |
| `--color-secondary` | Lavender | Secondary accent |
| `--color-secondary-content` | Crust / Base | Text on secondary |
| `--color-accent` | Rosewater | Warm accent |
| `--color-accent-content` | Crust / Base | Text on accent |
| `--color-neutral` | Surface 0 | Neutral/card bg |
| `--color-neutral-content` | Subtext 0 | Text on neutral |
| `--color-info` | Blue | Info state |
| `--color-info-content` | Crust / Base | Text on info |
| `--color-success` | Green | Success state |
| `--color-success-content` | Crust / Base | Text on success |
| `--color-warning` | Yellow | Warning state |
| `--color-warning-content` | Crust / Base | Text on warning |
| `--color-error` | Red | Error state |
| `--color-error-content` | Crust / Base | Text on error |

## Hex Values by Flavor

### Mocha (dark)
Base: #1e1e2e, Mantle: #181825, Crust: #11111b, Text: #cdd6f4, Mauve: #cba6f7, Lavender: #b4befe, Rosewater: #f5e0dc, Surface0: #313244, Subtext0: #a6adc8, Blue: #89b4fa, Green: #a6e3a1, Yellow: #f9e2af, Red: #f38ba8

### Macchiato (dark)
Base: #24273a, Mantle: #1e2030, Crust: #181926, Text: #cad3f5, Mauve: #c6a0f6, Lavender: #b7bdf8, Rosewater: #f4dbd6, Surface0: #363a4f, Subtext0: #a5adcb, Blue: #8aadf4, Green: #a6da95, Yellow: #eed49f, Red: #ed8796

### Frappé (dark)
Base: #303446, Mantle: #292c3c, Crust: #232634, Text: #c6d0f5, Mauve: #ca9ee6, Lavender: #babbf1, Rosewater: #f2d5cf, Surface0: #414559, Subtext0: #a5adce, Blue: #8caaee, Green: #a6d189, Yellow: #e5c890, Red: #e78284

### Latte (light)
Base: #eff1f5, Mantle: #e6e9ef, Crust: #dce0e8, Text: #4c4f69, Mauve: #8839ef, Lavender: #7287fd, Rosewater: #dc8a78, Surface0: #ccd0da, Subtext0: #6c6f85, Blue: #1e66f5, Green: #40a02b, Yellow: #df8e1d, Red: #d20f39

## Steps
1. Add 4 `@plugin "./daisyui-theme.mjs" { ... }` blocks to `styles.css`
2. Theme names: `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `catppuccin-mocha`
3. Set `catppuccin-mocha` as `--prefersdark` (replaces DaisyUI's `dark` as system dark default)

## Depends on
- `een` (daisyui-theme.mjs must be downloaded first)