---
id: kt9
title: Frontmatter extraction and parsing
status: done
priority: P1
created: 2026-03-27T21:24:45.827444450Z
updated: 2026-03-27T22:00:25.927119537Z
tags:
- backend
depends_on:
- vuh
parent: z69
---

## Summary

Add YAML frontmatter extraction to the markdown pipeline. Strip the `---` fenced YAML block before rendering and parse it into a `Frontmatter` struct.

## Acceptance Criteria

- [ ] Frontmatter between opening and closing `---` is stripped before markdown rendering
- [ ] Parsed into `Frontmatter { title, tags, date }` struct (all fields `Option`)
- [ ] Notes without frontmatter render correctly (no crash, `None` frontmatter)
- [ ] Malformed frontmatter is handled gracefully (skip parsing, still render body)
- [ ] Unit tests for with/without/malformed frontmatter

## Implementation Notes

- Files: `src-tauri/src/markdown/mod.rs` (or a `frontmatter.rs` sub-module)
- Use `serde_yml` (already a dependency) for YAML deserialization
- The `Frontmatter` struct should live in `granit-types` so the frontend can use it too
- Rendering function signature evolves: `render(markdown: &str) -> RenderedNote`

## Testing

- Test: frontmatter present → parsed correctly, body rendered without YAML
- Test: no frontmatter → `None`, full content rendered
- Test: malformed YAML → `None` frontmatter, body still renders