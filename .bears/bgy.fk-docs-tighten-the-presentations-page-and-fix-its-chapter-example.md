---
id: bgy.fk
title: "docs: tighten the presentations page and fix its chapter example"
status: open
priority: P2
created: "2026-11-26T23:42:04.942979132Z"
updated: "2026-11-26T23:42:04.942979132Z"
tags:
  - docs
  - presentation
---

## Summary

`docs/content/wiki/presentations.md` is accurate on the DOM contract, splitting rules and keys. Its one broken example breaks the default templates, and the page is about 30% longer than it needs to be.

## Acceptance Criteria

- [ ] The "chapter header" example no longer styles `.slide:not(:first-child)::before`. Both default templates use `.slide::before` for the progress pill (`cave/presentations/default-dark.css`), so pasting the example turns the pill into a blob. Use another hook, or say that the pill rule must be removed first.
- [ ] Drop the "same HTML the reader renders" claims: slides use plain pulldown-cmark output, so alerts become `<blockquote class="markdown-alert-*">`, unlike the reader's markup (`markdown/presentation.rs`, `markdown/render.rs`).
- [ ] Heading classes must be valid CSS identifiers, and duplicates are dropped (`presentation.rs`).
- [ ] The window also reloads, or closes, after the agent edits or deletes the presented note (`commands/presentation.rs`).
- [ ] Cut the following:
  - the progress-dot formula
  - the "A title-slide style" example, which repeats an earlier one
  - the paragraph that repeats agent-tools-and-rag
  - the Mermaid bullet, shortened to two sentences
- [ ] Merge "How a template is applied" with "The canvas", before the examples.
- [ ] The DOM contract bullets, the layouts section and the keyboard table stay, unchanged in substance.

## Implementation Notes

Load the `aphid-content` skill. `CLAUDE.md` names this page as the public home of the slide contract.