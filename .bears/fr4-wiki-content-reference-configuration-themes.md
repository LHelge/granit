---
id: fr4
title: "Wiki content: Reference (configuration, themes)"
status: open
priority: P2
created: "2026-06-12T11:38:13.138122706Z"
updated: "2026-06-12T11:38:13.138122706Z"
tags:
  - docs
  - content
depends_on:
  - jfv
parent: sm3
---

## Summary

The two `category: Reference` wiki pages.

## Acceptance Criteria

- [ ] `wiki/configuration.md` — full `.granit/config.yml` reference: sidebar, theme, fonts, daily notes, agent/provider settings; note that config is cave-local and the last-open cave path is stored separately. Verify every key against `granit-types` source (the config structs) rather than trusting README.
- [ ] `wiki/themes.md` — available DaisyUI/Catppuccin theme variants (Catppuccin Latte/Frappé/Macchiato/Mocha, Gruvbox, Tokyo Night, Rosé Pine, One Dark), per-surface font settings.
- [ ] configuration.md includes a fenced ```yml example config — doubles as the syntax-highlight test page for the theme.

## Implementation Notes

- Source of truth for keys: `granit-types/` config types + README "Config model". Read the actual structs.
- Theme list source: `styles.css` DaisyUI theme definitions.