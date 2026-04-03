---
id: pba
title: Define theme data model and Catppuccin theme files
status: open
priority: P1
created: 2026-04-02T23:09:52.730254257Z
updated: 2026-04-02T23:09:52.730254257Z
tags:
- core
parent: njk
---

## Summary
Define the theme data model and create the four Catppuccin theme files that will be compiled into the binary.

## Acceptance Criteria
- [ ] Theme struct in `granit-types` with all 26 Catppuccin color fields + metadata (name, id/slug, dark flag)
- [ ] Four JSON/YAML theme files under a `themes/` directory (latte, frappe, macchiato, mocha)
- [ ] Hex color values sourced from the official Catppuccin palette
- [ ] `include_str!` or `include_bytes!` to embed theme files at compile time
- [ ] Theme registry that deserializes and exposes all built-in themes
- [ ] Unit tests for deserialization and registry lookup

## Implementation Notes
- Add theme types to `granit-types/src/` (new `theme.rs` module)
- Theme files go in `themes/catppuccin-latte.json`, etc.
- Colors from: https://github.com/catppuccin/palette  
- Each color stored as hex string (e.g. `"#1e1e2e"`)
- Registry returns `&[Theme]` and lookup by id