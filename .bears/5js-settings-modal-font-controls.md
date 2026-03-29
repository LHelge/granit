---
id: 5js
title: Settings modal font controls
status: done
priority: P1
created: 2026-03-27T21:37:56.127460951Z
updated: 2026-03-29T13:15:44.773091617Z
tags:
- frontend
depends_on:
- hvc
parent: bp2
---

## Summary

Add font family and font size fields to the settings modal so the user can configure editor appearance.

## Acceptance Criteria

- [ ] New "Editor" fieldset in the settings modal with font family and font size inputs
- [ ] Font family: text input (e.g., `"monospace"`, `"JetBrains Mono"`, `"Inter"`)
- [ ] Font size: number input with reasonable min/max (8–32)
- [ ] Values initialized from current config on modal open
- [ ] Saved alongside agent settings via the existing `save_config` flow

## Implementation Notes

- Files: `src/app/components/settings_modal.rs`
- Add local signals for `editor_font` and `editor_font_size`, same pattern as `provider`/`model`
- The `save_config` IPC call needs to include the editor config — may need to update the command signature or add a separate command

## Testing

- Manual: open settings, change font/size, save, verify config.yml updated