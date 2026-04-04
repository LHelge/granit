---
id: 9fd
title: Adopt DaisyUI 5 component classes across frontend
type: epic
status: open
priority: P1
created: 2026-04-04T21:13:16.228166660Z
updated: 2026-04-04T21:13:16.228166660Z
---

## Scope

Replace hand-rolled Tailwind utility patterns with DaisyUI 5 component classes throughout the Leptos frontend. This reduces CSS verbosity, improves visual consistency, and ensures all components respect the active DaisyUI theme automatically.

## Acceptance Criteria

- [ ] All buttons use `btn` variants instead of manual bg/border/padding/hover classes
- [ ] All text inputs use `input` class, textareas use `textarea` class
- [ ] Frontmatter tags and theme labels use `badge` variants
- [ ] Toast notifications use `toast` + `alert` components
- [ ] Settings modal uses `modal` / `modal-box` / `modal-backdrop`
- [ ] Agent chat messages use `chat` / `chat-bubble` components
- [ ] Context menu uses `menu` component
- [ ] Loading states use `loading` component
- [ ] Dividers use `divider` class
- [ ] Settings fieldsets use DaisyUI `fieldset` component
- [ ] Toolbar buttons use `tooltip` component
- [ ] Dropdowns evaluated for DaisyUI conversion where feasible
- [ ] No visual regressions — app looks the same or better
- [ ] `cargo fmt && cargo clippy` pass after all changes

## Phases

1. **Foundation** (P1): Buttons, Inputs, Textarea, Badges, Toasts — highest code reduction
2. **Structure** (P1): Modal, Chat Bubbles, Menu — structural improvements
3. **Polish** (P2): Tooltips, Loading, Dividers, Fieldsets, Dropdowns — finishing touches
