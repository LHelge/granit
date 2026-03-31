---
id: 2cf
title: Wiki-link resolution should respect code blocks
status: open
priority: P2
created: 2026-03-31T16:33:19.413375244Z
updated: 2026-03-31T16:33:19.413375244Z
tags:
- backend
- bug
parent: 4cm
---

## Summary
`resolve_wiki_links` in markdown/mod.rs uses `find("[[")` / `find("]]")` on raw text. This breaks if double brackets appear inside code blocks or inline code spans.

## Acceptance Criteria
- [ ] Wiki-links inside fenced code blocks are NOT resolved
- [ ] Wiki-links inside inline code are NOT resolved
- [ ] Normal text wiki-links still resolve correctly
- [ ] Add tests for `[[link]]` inside code blocks

## Implementation Notes
- Files: `src-tauri/src/markdown/mod.rs`
- Option A: Run wiki-link resolution on the pulldown-cmark event stream (only process `Event::Text` nodes)
- Option B: Pre-scan for code blocks/inline code and mask them before string search
- Option A is more robust and aligns with the existing pulldown-cmark pipeline

## Testing
- Test: fenced code containing `[[not-a-link]]` → rendered as-is, not linked
- Test: inline `` `[[not-a-link]]` `` → not linked