---
id: bp2
title: Editor and chat font configuration
type: epic
status: open
priority: P2
created: 2026-03-27T21:37:34.875669640Z
updated: 2026-03-29T21:02:23.988684114Z
---

## Scope

Allow the user to configure font family and font size separately for markdown editing, reading/preview, and agent chat. Settings are persisted in the global config and applied via reactive CSS.

## Design

Settings modal redesigned with a sidebar layout:
- Left: list of settings sections (Markdown, Reading, Agent)
- Right: content pane showing the selected section's settings

### Sections

- **Markdown**: Font family + font size for the markdown editor textarea
- **Reading**: Font family + font size for the rendered HTML preview
- **Agent**: Font family + font size for agent chat, plus existing provider/model settings

## Acceptance Criteria

- [ ] Settings modal has sidebar + content pane layout
- [ ] Three font configs (markdown, reading, agent) in config system
- [ ] Font controls in each settings section
- [ ] Font configs applied to editor, preview, and chat respectively
- [ ] Defaults are sensible (monospace for editor, proportional for preview/chat)
- [ ] Changes apply immediately on save without restarting the app