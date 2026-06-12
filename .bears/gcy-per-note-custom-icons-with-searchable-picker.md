---
id: gcy
title: Per-note custom icons with searchable picker
type: epic
status: done
priority: P1
created: 2026-04-03T14:10:07.368511904Z
updated: 2026-04-03T21:08:43.542133890Z
tags:
- notes
- frontend
- backend
---

## Scope

Add optional per-note custom icons stored in markdown frontmatter, expose a curated searchable icon picker in the editor frontmatter UI, and render the chosen icon in the tree view with `LuFileText` as the fallback.

## Acceptance Criteria
- [ ] Notes support an optional `icon` frontmatter field stored as a normalized app string.
- [ ] Note metadata returned to the frontend includes the optional icon so the tree can render it.
- [ ] The editor frontmatter UI includes an icon picker with a search field above a scrollable icon grid.
- [ ] Users can clear the icon selection and fall back to `LuFileText`.
- [ ] Unknown or missing icon ids render safely with the default file icon.
- [ ] Backend tests cover frontmatter parsing/persistence for icons.
- [ ] Manual verification confirms icon persistence across save, reopen, rename, and move flows.

## Implementation Notes
- Keep backend as the source of truth for metadata.
- Store `icon` as a normalized string, not a raw `icondata_lu` constant name.
- Use a curated Lucide subset with custom search tags rather than the full icon set.
- Reuse the existing backdrop-assisted popover pattern for the picker UI.
