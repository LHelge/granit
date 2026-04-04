---
id: uqp
title: DaisyUI 5 theme system — replace custom theme with daisyUI themes
type: epic
status: done
priority: P1
created: 2026-04-04T20:02:41.494942590Z
updated: 2026-04-04T20:54:38.731036765Z
---

## Scope

Replace the existing custom theme system (19 hex-color `Theme` struct, `ThemeRegistry`, `apply_theme()` CSS-var injection) with DaisyUI 5's built-in theme system using `data-theme` on `<html>`. Enable all 35 built-in themes, persist selection in config, and migrate ~180 custom color token usages across 20 frontend files to daisyUI semantic color classes.

## Acceptance Criteria

- [ ] DaisyUI 5 standalone JS plugin integrated into Tailwind build pipeline
- [ ] All 35 built-in daisyUI themes available
- [ ] Theme selection persisted in `AppConfig.theme` and restored on startup without FOUC
- [ ] `data-theme` attribute set reactively on `<html>` from Leptos
- [ ] Theme picker in settings dialog shows available themes with live preview
- [ ] All custom `--theme-*` CSS variables, `@theme` color tokens, and `Theme`/`ThemeRegistry` backend code removed
- [ ] All ~180 custom color class usages migrated to daisyUI equivalents
- [ ] `cargo fmt && cargo clippy && cargo test` pass
- [ ] Prose/markdown styling works with daisyUI themes