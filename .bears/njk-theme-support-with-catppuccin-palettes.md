---
id: njk
title: Theme support with Catppuccin palettes
type: epic
status: done
priority: P2
created: 2026-04-02T23:08:54.278899191Z
updated: 2026-04-03T23:51:21.344996997Z
---

## Scope

Add a theming system to Granit. Themes are defined as data files (JSON/YAML) in the project source tree and compiled into the binary. The UI uses CSS custom properties (variables) mapped from the active theme, replacing all hardcoded Tailwind color classes.

Ship with the four **Catppuccin** flavors as built-in themes:
- 🌻 Latte (light)
- 🪴 Frappé (dark)
- 🌺 Macchiato (dark)
- 🌿 Mocha (dark)

Reference: https://github.com/catppuccin/catppuccin
Style guide: https://github.com/catppuccin/catppuccin/blob/main/docs/style-guide.md

## Design Decisions

### Theme data format
Each theme file defines the 26 Catppuccin palette colors (rosewater, flamingo, pink, mauve, red, maroon, peach, yellow, green, teal, sky, sapphire, blue, lavender, text, subtext1, subtext0, overlay2, overlay1, overlay0, surface2, surface1, surface0, base, mantle, crust) plus metadata (name, dark/light flag).

### CSS custom properties
The backend serves the active theme's colors. The frontend maps them to CSS custom properties (`--color-base`, `--color-text`, etc.) and Tailwind references these via `@theme` in `styles.css`. All hardcoded `stone-*` and `red-*` classes are replaced with semantic theme tokens.

### Semantic mapping (from Catppuccin style guide)
- **Background pane**: Base
- **Secondary panes** (sidebar, titlebar): Mantle, Crust
- **Surface elements** (buttons, cards): Surface 0/1/2
- **Primary text**: Text
- **Secondary text**: Subtext 0/1
- **Subtle/muted text**: Overlay 1
- **Links, tags**: Blue
- **Errors**: Red
- **Warnings**: Yellow
- **Success**: Green
- **Accent/cursor**: Rosewater

### Theme persistence
Active theme is stored in the global config (`~/.config/granit/config.yml`) and optionally overridden per-cave.

## Acceptance Criteria

- [ ] Four Catppuccin themes compiled into the binary
- [ ] Theme struct and registry in the backend
- [ ] CSS custom properties driven by active theme
- [ ] All frontend components use theme variables (no hardcoded colors)
- [ ] Theme selection in settings UI
- [ ] Active theme persisted in config
- [ ] Switching themes updates the UI immediately
- [ ] Light theme (Latte) works correctly (text/bg contrast inverted)