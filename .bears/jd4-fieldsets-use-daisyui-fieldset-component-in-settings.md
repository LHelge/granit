---
id: jd4
title: 'Fieldsets: Use DaisyUI fieldset component in settings'
status: done
priority: P2
created: 2026-04-04T21:15:07.856320402Z
updated: 2026-04-04T21:57:49.557000924Z
tags:
- frontend
depends_on:
- exw
parent: 9fd
---

## Summary

Replace the custom fieldset+legend styling in settings sections with DaisyUI `fieldset` component and `fieldset-legend` class.

## Acceptance Criteria

- [ ] Settings `<fieldset>` elements use `fieldset` DaisyUI class
- [ ] Settings `<legend>` elements use `fieldset-legend` class
- [ ] Description labels use `label` class from DaisyUI fieldset
- [ ] Visual appearance matches or improves on current styling

## Files to Modify

- `src/app/components/settings/agent.rs` — Agent fieldset
- `src/app/components/settings/markdown.rs` — Markdown fieldset
- `src/app/components/settings/reading.rs` — Reading fieldset
- `src/app/components/settings/notes.rs` — Notes fieldset
