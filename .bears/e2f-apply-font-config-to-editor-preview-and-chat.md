---
id: e2f
title: Apply font config to editor, preview, and chat
status: open
priority: P1
created: 2026-03-27T21:38:06.579772716Z
updated: 2026-03-27T21:38:06.579772716Z
tags:
- frontend
depends_on:
- 5js
parent: bp2
---

## Summary

Apply the configured font family and font size to the editor textarea, markdown preview pane, and agent chat panel using reactive inline styles.

## Acceptance Criteria

- [ ] Editor textarea uses configured font family and size
- [ ] Preview pane (rendered markdown) uses configured font family and size
- [ ] Agent chat messages use configured font family and size
- [ ] Changes take effect immediately when config signal updates (no page reload)
- [ ] Defaults look good out of the box (monospace for editor textarea, system/proportional for preview and chat is also acceptable)

## Implementation Notes

- Files: `src/app/components/editor.rs`, `src/app/components/agent_panel.rs`
- The `AppConfig` signal is already available at the app level — pass `EditorConfig` (or the whole config signal) down to the editor and agent components
- Apply via `style:font-family` and `style:font-size` attributes in Leptos `view!` macros
- Consider whether editor and preview/chat should share the same font or have separate configs — start with one shared config, split later if needed

## Testing

- Manual: change font in settings, verify all three areas update
- Test with a custom font name, verify fallback works