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