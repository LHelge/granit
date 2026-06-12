---
id: "7t6"
title: Code highlighting (hl-*), GitHub alerts, responsive polish
status: done
priority: P2
created: "2026-06-12T11:37:27.638552339Z"
updated: "2026-06-12T12:20:58.315613821Z"
tags:
  - docs
  - theme
  - css
depends_on:
  - na8
  - u8w
parent: f9w
---

## Summary

Finish the content styling: syntax-highlight palette, alert callouts, mermaid sizing, and a responsive pass.

## Acceptance Criteria

- [ ] Code blocks: `--base-300` background, `--radius`, `overflow-x: auto`, monospace stack.
- [ ] `hl-*` classes mapped to Catppuccin Mocha (full class list in `.claude/skills/aphid-theme/SKILL.md`): keyword `#cba6f7`, string `#a6e3a1`, comment `#6c7086`, type `#f9e2af`, function `#89b4fa`, number `#fab387`, operator `#94e2d5`, punctuation `#9399b2`, variable `#cdd6f4`, attribute `#f9e2af`, tag `#cba6f7`, entity `#89b4fa`.
- [ ] GitHub alerts (`> [!NOTE]/[!TIP]/[!IMPORTANT]/[!WARNING]/[!CAUTION]`) styled as left-bordered callout cards using `--info/--success/--primary/--warning/--error` respectively.
- [ ] Mermaid `<pre class="mermaid">` blocks centered with sane max-width.
- [ ] Responsive pass at 375/768/1280px: no horizontal page scroll, header fits, tables scroll within the prose column.

## Implementation Notes

- Build the site and inspect emitted HTML for the alert markup aphid produces (class names/structure) before writing selectors.
- Include a test page locally (or use the configuration wiki page) with rust + yaml + toml code fences and every alert type to verify against — don't commit a scratch page.