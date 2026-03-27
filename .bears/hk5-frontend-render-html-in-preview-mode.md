---
id: hk5
title: 'Frontend: render HTML in preview mode'
status: open
priority: P1
created: 2026-03-27T21:25:58.693513335Z
updated: 2026-03-27T21:25:58.693513335Z
tags:
- frontend
depends_on:
- 26a
parent: z69
---

## Summary

Update the editor's preview mode to call `render_note` and display the returned HTML instead of raw markdown text. Style the rendered output with Tailwind's prose utilities.

## Acceptance Criteria

- [ ] Preview mode calls `ipc::render_note(slug)` when a note is selected or content changes
- [ ] Rendered HTML is displayed using Leptos `inner_html` (or equivalent) inside the prose container
- [ ] `prose prose-invert` Tailwind classes applied for consistent dark-theme styling
- [ ] Headings, bold, italic, lists, code blocks, links, tables, strikethrough, task lists all render correctly
- [ ] Wiki-links are clickable and navigate to the target note (or show as broken links)
- [ ] No regressions: edit mode still works with raw textarea

## Implementation Notes

- Files: `src/app/components/editor.rs`, `src/app/ipc.rs`
- Currently the preview fallback is: `<p class="text-stone-300 whitespace-pre-wrap">{move || content.get()}</p>`
- Replace with: `<div class="prose prose-invert max-w-none" inner_html=move || rendered_html.get() />`
- Add a `rendered_html` signal that updates when the note changes
- Consider calling `render_note` reactively when `active_note` changes (not on every keystroke)
- For wiki-link navigation: intercept click events on internal links and update `active_note`

## Edge Cases

- Empty note → empty preview (no crash)
- Note with only frontmatter → empty body preview
- Very large notes → ensure no UI freeze (render is async via IPC)

## Testing

- Manual: open a note with various markdown elements, verify rendering
- Verify edit ↔ preview toggling still works
- Verify auto-save on note switch still works