---
id: z69
title: Markdown rendering in preview mode
type: epic
status: done
priority: P1
created: 2026-03-27T21:24:24.895817170Z
updated: 2026-03-27T23:28:04.314385866Z
---

## Scope

Replace the raw-text preview in the editor with properly rendered HTML from pulldown-cmark. The backend gains a markdown processing module; the frontend displays the rendered output.

## Current State

- Preview mode shows raw markdown in a `<p whitespace-pre-wrap>` tag
- No `pulldown-cmark` dependency, no `markdown/` module in the backend
- Frontend receives raw `.md` content from `read_note`

## Acceptance Criteria

- [ ] Preview mode renders headings, bold, italic, lists, code blocks, links, tables, strikethrough, and task lists
- [ ] YAML frontmatter is stripped before rendering (not shown in preview)
- [ ] Wiki-links (`[[note-name]]`) are resolved and rendered as clickable links
- [ ] Rendered HTML is styled consistently with the dark theme (Tailwind prose-invert)
- [ ] Backend has a tested `markdown/` module with the full pipeline