---
id: dpg
title: Final cleanup and cross-theme visual verification
status: done
priority: P2
created: 2026-04-04T20:04:40.204506603Z
updated: 2026-04-04T20:54:38.730859769Z
tags:
- qa
depends_on:
- 46x
parent: uqp
---

## Summary

Final verification pass — ensure everything works end-to-end after all previous tasks, clean up any remaining references, and verify across themes.

## Implementation Notes

1. **Search for stale references**:
   - `grep -r "theme-window\|theme-panel\|theme-card\|theme-fg\|theme-edge\|apply_theme\|ThemeRegistry\|ThemeMeta\|builtin_themes" src/ src-tauri/ granit-types/`
   - Remove any leftovers

2. **Verify config migration**:
   - Existing user configs have `theme: "default"` or `theme: "mocha"` — these won't match daisyUI names
   - Add fallback in `set_daisy_theme()`: if theme name is unknown, fall back to `"dark"`
   - Add similar fallback in backend `set_active_theme` command

3. **Run full test suite**:
   - `cargo fmt --all`
   - `cargo clippy --workspace`
   - `cargo test -p granit`
   - `cargo test -p granit-types`

4. **Visual smoke test**:
   - Test themes: light, dark, nord, dracula, cupcake, coffee (mix of light/dark)
   - Verify: sidebar, editor, agent panel, settings modal, tree view, context menus
   - Verify prose/markdown rendering looks correct
   - Verify broken-link styling

## Files to Check
- All files from previous tasks
- Any other files that might reference old theme types

## Acceptance Criteria
- [ ] No stale theme references anywhere in codebase
- [ ] Config migration handles old theme names gracefully
- [ ] All tests pass
- [ ] Visual appearance is correct across at least 4 different themes (2 light, 2 dark)
- [ ] `cargo fmt && cargo clippy && cargo test` clean