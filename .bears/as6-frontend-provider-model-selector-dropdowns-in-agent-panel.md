---
id: as6
title: 'Frontend: provider + model selector dropdowns in agent panel'
status: open
priority: P1
created: 2026-03-31T21:51:02.877202626Z
updated: 2026-03-31T21:51:02.877202626Z
tags:
- frontend
depends_on:
- hq2
parent: c56
---

## Summary

Add two dropdowns above the chat message input in the agent panel: one for selecting the active provider, one for selecting the model. Models are fetched dynamically when a provider is selected.

## Acceptance Criteria

- [ ] Provider dropdown shows display names of all configured providers (from `list_providers` command)
- [ ] Model dropdown shows models fetched from `list_models` for the selected provider
- [ ] Selecting a provider calls `select_provider(index)`, then refreshes the model list
- [ ] Selecting a model calls `select_model(model_id)`, which resets the agent
- [ ] Both dropdowns show loading/error states (e.g., spinner while fetching models, error if provider unreachable)
- [ ] Chat history clears when provider or model changes (existing identity-tracking behavior)
- [ ] Dropdowns are disabled while streaming a response
- [ ] Layout: provider dropdown on the left, model dropdown on the right, both above the textarea input

## Implementation Notes

- File: `src/app/components/agent_panel.rs`
- Use `spawn_local` to call IPC commands on mount and on selection change
- Provider list can be fetched once on mount + after settings save
- Model list refreshes when provider selection changes
- Current selection should reflect config values (pre-select based on `selected_provider` and last-used model)
- Style with Tailwind: `<select>` elements with consistent styling, flex row layout