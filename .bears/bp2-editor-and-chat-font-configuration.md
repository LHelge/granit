---
id: bp2
title: Editor and chat font configuration
type: epic
status: open
priority: P2
created: 2026-03-27T21:37:34.875669640Z
updated: 2026-03-27T21:37:34.875669640Z
---

## Scope

Allow the user to configure font family and font size for the editor (textarea), preview (rendered HTML), and agent chat panel. Settings are persisted in the global config and applied via reactive CSS.

## Current State

- No font/size configuration exists
- Editor uses Tailwind defaults (`font-mono text-sm` for textarea, prose for preview)
- Agent panel uses Tailwind default font sizes
- Settings modal only has agent config fields

## Acceptance Criteria

- [ ] New `EditorConfig { font_family, font_size }` in config system
- [ ] Settings modal exposes font family and font size controls
- [ ] Editor textarea, preview pane, and agent chat all respect the configured font/size
- [ ] Defaults are sensible (monospace for editor, proportional for preview/chat)
- [ ] Changes apply immediately on save without restarting the app