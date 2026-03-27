---
id: qvu
title: Wiki-link resolution
status: done
priority: P1
created: 2026-03-27T21:25:27.728728237Z
updated: 2026-03-27T22:13:53.845690889Z
tags:
- backend
depends_on:
- kt9
parent: z69
---

## Summary

Implement wiki-link (`[[note-name]]`) detection and resolution in the markdown pipeline. Before passing content to pulldown-cmark, find all `[[...]]` patterns and replace them with standard markdown links pointing to the resolved note.

## Acceptance Criteria

- [ ] Regex finds all `[[note-name]]` patterns in the markdown body (after frontmatter stripping)
- [ ] Each wiki-link is resolved by searching the cave for a matching `.md` file (case-insensitive filename match)
- [ ] Resolved links become standard markdown links: `[note-name](slug)`
- [ ] Unresolved links are rendered distinctly (e.g., with a CSS class for broken links)
- [ ] The list of outgoing links (resolved filenames) is collected and returned in `RenderedNote`
- [ ] Unit tests cover: resolved link, unresolved link, multiple links, no links

## Implementation Notes

- Files: `src-tauri/src/markdown/mod.rs` (or `wikilink.rs` sub-module)
- The render function needs access to the list of note filenames in the cave to resolve links
- Signature evolves: `render(markdown: &str, cave_notes: &[&str]) -> RenderedNote`
- Use a simple regex like `\[\[([^\]]+)\]\]` to find wiki-links
- Add `regex` crate if not already a dependency

## Testing

- Test: `[[existing-note]]` → `<a href="existing-note">existing-note</a>`
- Test: `[[missing-note]]` → rendered with broken-link indicator
- Test: multiple wiki-links in one document