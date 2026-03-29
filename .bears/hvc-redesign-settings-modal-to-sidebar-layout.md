---
id: hvc
title: Redesign settings modal to sidebar layout
status: done
priority: P1
created: 2026-03-27T21:37:46.400844004Z
updated: 2026-03-29T21:13:52.883684522Z
tags:
- frontend
parent: bp2
---

## Summary

Restructure the settings modal to be wider with a two-column layout: a section list on the left and a content pane on the right. Move existing agent settings into the "Agent" section. Add empty placeholder sections for "Markdown" and "Reading" (font controls added in later tasks).

## Acceptance Criteria

- [ ] Modal is wider (e.g. `w-[640px]` or similar)
- [ ] Left side: vertical list of section names (Markdown, Reading, Agent)
- [ ] Clicking a section highlights it and shows its content on the right
- [ ] Agent section contains all existing agent settings (provider, model, base URL, API key)
- [ ] Markdown and Reading sections show placeholder text for now
- [ ] Save/Cancel buttons remain at the bottom
- [ ] Active section stored in a local signal

## Implementation Notes

- File: `src/app/components/settings_modal.rs`
- Use a local `signal` for the active section (default to "Agent" since it has content)
- Wrap the existing form fieldset in a conditional `Show` or match on active section
- No backend changes needed

## Testing

- Manual: open settings, verify sidebar layout, switch sections, save agent settings still works