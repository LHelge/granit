---
id: axu
title: Migrate all custom color tokens to daisyUI semantic colors
status: done
priority: P1
created: 2026-04-04T20:04:10.389316467Z
updated: 2026-04-04T20:44:13.299720584Z
tags:
- frontend
depends_on:
- v89
parent: uqp
---

## Summary

Migrate all ~180 custom color class usages across 20 frontend files from the old semantic tokens (`bg-window`, `text-fg`, `border-edge`, etc.) to daisyUI equivalents (`bg-base-100`, `text-base-content`, `border-base-300`, etc.).

## Color Mapping

| Old token | DaisyUI replacement | Notes |
|---|---|---|
| `bg-window` | `bg-base-100` | Deepest background |
| `bg-panel` | `bg-base-200` | Panel/sidebar background |
| `bg-card` | `bg-base-300` | Elevated surfaces, modals |
| `bg-item-hover` | `hover:bg-base-300` or `bg-base-content/10` | Hover states |
| `bg-item-active` | `bg-base-content/15` or `bg-primary/10` | Active/pressed |
| `bg-highlight` | `bg-base-content/20` | Strongest BG highlight |
| `text-fg` | `text-base-content` | Primary text |
| `text-fg-secondary` | `text-base-content/80` | Secondary text |
| `text-fg-muted` | `text-base-content/60` | Muted text, icons |
| `text-fg-faint` | `text-base-content/40` | Placeholders, hints |
| `border-edge` | `border-base-300` | Default border |
| `border-edge-subtle` | `border-base-200` or `border-base-content/10` | Subtle dividers |
| `border-edge-hover` | `border-base-content/30` | Hover border |
| `border-edge-focus` | `border-primary` or `border-base-content/40` | Focus ring |
| `bg-accent` / `text-accent` | `bg-primary` / `text-primary` | Primary accent |
| `bg-error` / `text-error` | `bg-error` / `text-error` | Same name in daisyUI |
| `bg-success` / `text-success` | `bg-success` / `text-success` | Same name |
| `bg-warning` / `text-warning` | `bg-warning` / `text-warning` | Same name |

## Files to Migrate (by priority — highest usage first)

1. `src/app/components/agent_panel.rs` (~18 usages)
2. `src/app/components/settings/agent.rs` (~17 usages)
3. `src/app/mod.rs` (~14 usages)
4. `src/app/components/settings/mod.rs` (~11 usages)
5. `src/app/components/settings/font_picker.rs` (~10 usages)
6. `src/app/components/cave_selector.rs` (~9 usages)
7. `src/app/components/provider_selector.rs` (~8 usages)
8. `src/app/components/model_selector.rs` (~8 usages)
9. `src/app/components/editor/mod.rs` (~8 usages)
10. `src/app/components/editor/icon_picker.rs` (~7 usages)
11. `src/app/components/settings/theme.rs` (~6 usages)
12. `src/app/components/editor/reader.rs` (~5 usages)
13. `src/app/components/editor/frontmatter.rs` (~4 usages)
14. `src/app/components/tree_view/note_node.rs` (~4 usages)
15. `src/app/components/tree_view/folder_node.rs` (~3 usages)
16. `src/app/components/tree_view/context_menu.rs` (~3 usages)
17. `src/app/components/tree_view/rename_input.rs` (~3 usages)
18. `src/app/components/settings/reading.rs` (~3 usages)
19. `src/app/components/settings/notes.rs` (~3 usages)
20. `src/app/components/settings/markdown.rs` (~3 usages)
21. `src/app/components/sidebar.rs` (~3 usages)

## Acceptance Criteria
- [ ] No `bg-window`, `text-fg`, `border-edge`, or any old custom color classes remain in `src/`
- [ ] All replaced with daisyUI semantic colors
- [ ] Visual appearance is reasonable across light and dark themes
- [ ] `cargo clippy` clean (no unused imports)

## Testing
- `cargo fmt`
- Visual inspection: switch between light, dark, nord, dracula themes — UI should look coherent
- No broken layouts or invisible text