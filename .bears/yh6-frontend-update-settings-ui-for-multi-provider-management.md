---
id: yh6
title: 'Frontend: update settings UI for multi-provider management'
status: open
priority: P2
created: 2026-03-31T21:51:15.440534520Z
updated: 2026-03-31T21:51:15.440534520Z
tags:
- frontend
depends_on:
- vyz
parent: c56
---

## Summary

Rework the settings modal's agent section to support adding, editing, and removing multiple providers. Each provider shows fields matching its enum variant (API key for Anthropic/Mistral, base URL for Ollama/Mistral). Remove all secrets-related UI.

## Acceptance Criteria

- [ ] Agent settings shows a list of configured providers (editable)
- [ ] "Add provider" button with provider type selector (Ollama, Anthropic, Mistral)
- [ ] Each provider row shows relevant fields (base_url, api_key) based on type
- [ ] Remove button for each provider (with confirmation if it's the selected one)
- [ ] API key fields use `type="password"` input (masked by default, toggle to reveal)
- [ ] Remove old single-provider fields (provider dropdown, model input, base_url input)
- [ ] Remove secret key input and `api_key_name()` helper
- [ ] Remove `get_secret`/`set_secret` IPC calls from frontend
- [ ] Save persists full provider list to config
- [ ] `max_history` setting remains as agent-level config
- [ ] Font settings unchanged

## Implementation Notes

- Files: `src/app/components/settings/mod.rs`, `src/app/ipc.rs` (remove secret IPC functions)
- Consider a sub-component for a single provider editor row
- Use Leptos signals for the mutable provider list in the form
- On save, call `save_config` with updated `AgentConfig` containing the full providers vector