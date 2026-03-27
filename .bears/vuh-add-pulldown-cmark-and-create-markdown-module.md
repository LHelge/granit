---
id: vuh
title: Add pulldown-cmark and create markdown module
status: done
priority: P1
created: 2026-03-27T21:24:34.456433015Z
updated: 2026-03-27T21:46:42.637443222Z
tags:
- backend
parent: z69
---

## Summary

Add `pulldown-cmark` to the backend and create the `src-tauri/src/markdown/` module with the core rendering pipeline.

## Acceptance Criteria

- [ ] `pulldown-cmark` added via `cargo add pulldown-cmark`
- [ ] `src-tauri/src/markdown/mod.rs` created with a public `render(markdown: &str) -> String` function
- [ ] Parser options enabled: tables, strikethrough, task lists
- [ ] Unit tests pass for basic markdown (headings, bold, italic, lists, code blocks, links)
- [ ] Module declared in `src-tauri/src/lib.rs`

## Implementation Notes

- Files: create `src-tauri/src/markdown/mod.rs`, update `src-tauri/src/lib.rs`
- Follow the Options pattern from the markdown-processing instructions
- Keep this task focused on the core pulldown-cmark pipeline — no frontmatter or wiki-links yet

## Testing

- Unit tests: render basic markdown elements and assert expected HTML output
- `cargo fmt && cargo clippy && cargo test` must pass