---
id: zem.sp
title: Update the docs site for the GPUI app
status: open
priority: P2
created: "2026-11-26T21:10:43.317023Z"
updated: "2026-11-26T21:10:43.317023Z"
tags:
  - gpui
  - port
  - docs
depends_on:
  - zem.8h
---

## Summary

The docs site (`docs/`, aphid) describes the Tauri app: reader and edit modes, DaisyUI themes, presentations, installation. Rewrite what changed for the GPUI app.

## Acceptance Criteria

- [ ] Every page under `docs/content/` checked; pages describing reader/edit mode, themes, the explorer, the agent chat and installation match the new app
- [ ] Removed features (per the mermaid and presentations decision) are removed or marked as gone, including the presentation template DOM contract page
- [ ] New behaviour documented: live-preview editing, `[[` completion, a plain click following links, streamed chat formatting
- [ ] Screenshots replaced
- [ ] Build-from-source instructions are cargo-only

## Implementation Notes

- Use the `aphid-content` skill for frontmatter and content conventions.

## Testing

- The docs site builds locally and links resolve
## Decisions (owner, 2026-11-28)

- Presentations and mermaid diagrams are dropped from the app (`zem.t7`): remove `presentations.md` with the template DOM contract, and what other pages say about diagrams.
- Linux ships a `.deb` and an AppImage, no `.rpm` (`zem.nb`): remove the rpm instructions, and use the package names after the cutover rename (`granit_<version>_amd64.deb`, `granit_<version>_x86_64.AppImage`).
- Updates stay silent, with the release notes shown after the restart (`zem.8j`).
