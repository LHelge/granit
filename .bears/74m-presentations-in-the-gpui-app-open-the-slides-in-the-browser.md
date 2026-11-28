---
id: 74m
title: "Presentations in the GPUI app: open the slides in the browser"
status: open
priority: P3
created: "2026-11-28T11:21:15.120428442Z"
updated: "2026-11-28T11:21:15.120428442Z"
tags:
  - gpui
  - presentations
---

## Summary

Presentations are dropped from the GPUI app for now (decision in `zem.t7`, 2026-11-28): the `presentation` frontmatter field and `.granit/presentations/` stay untouched on disk, and the slide rendering stays in `granit-core` (`markdown/presentation.rs`, `cave/presentations.rs`). If they are wanted back, the idea on record is to write the rendered slide page to a temporary folder and open it in the system browser.

## Acceptance Criteria

- [ ] A note with a `presentation` template can be presented from the GPUI app: its slides open in the system browser
- [ ] Existing templates work unchanged (the slide DOM and CSS custom-property contract is kept)
- [ ] Images of the note and assets of the template resolve in the exported page
- [ ] The docs describe it

## Implementation Notes

- No presentation window and no `granit://` scheme in the GPUI app: the page must be self-contained or reference files by path.
- Known losses against the Tauri window: no reload on note or template save, and no mermaid on slides (the mermaid bundle is built with Node, which the cutover removes).
- Decide first whether the feature is used enough to bring back.

## Testing

- Core tests for the exported page; a manual check in a browser with a default and a custom template