---
id: 8wf
title: 'Inputs: Replace manual input styles with input classes'
status: done
priority: P1
created: 2026-04-04T21:13:58.642377882Z
updated: 2026-04-04T21:39:42.550629655Z
tags:
- frontend
parent: 9fd
---

## Summary

Replace the repeated hand-rolled input styling pattern (`bg-base-100 border border-base-content/20 rounded px-3 py-1.5 text-sm ...`) with DaisyUI `input input-bordered` classes.

## Acceptance Criteria

- [ ] All `<input type="text">`, `type="number"`, `type="password"` use `input input-bordered input-sm w-full`
- [ ] Ghost-style inputs (e.g. tag add input in frontmatter) use `input input-ghost input-xs`
- [ ] Search inputs (font picker, icon picker) use `input input-bordered input-sm`
- [ ] Color variants used where appropriate (e.g. `input-error` for validation)
- [ ] Focus ring handled by DaisyUI (remove manual `focus:border-primary`)

## Files to Modify

- `src/app/components/settings/agent.rs` — font size, provider name, base URL, API key inputs
- `src/app/components/settings/markdown.rs` — font size input
- `src/app/components/settings/reading.rs` — font size input
- `src/app/components/settings/notes.rs` — daily note folder input
- `src/app/components/settings/font_picker.rs` — search input, trigger button
- `src/app/components/editor/frontmatter.rs` — tag add input
- `src/app/components/editor/icon_picker.rs` — search input
