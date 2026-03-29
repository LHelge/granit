---
id: mwp
title: 'Frontend: model selector in agent panel'
status: open
priority: P1
created: 2026-03-29T00:06:51.239905836Z
updated: 2026-03-29T00:06:51.239905836Z
tags:
- frontend
depends_on:
- y2f
parent: xd5
---

## Summary

Add a model selector dropdown in the agent panel, near the chat message input. The dropdown fetches available models from the backend and lets the user switch models on the fly.

## Acceptance Criteria

- [ ] Model dropdown appears above or beside the chat input area in the agent panel
- [ ] Dropdown populates by calling `list_models` IPC command
- [ ] Shows the current model from config as the selected value
- [ ] Selecting a different model calls `save_config` with the new model and resets the agent
- [ ] Loading state while fetching models (e.g., "Loading models...")
- [ ] Error state if provider is unreachable (show error text, allow retry)
- [ ] Refresh button or auto-refresh on panel mount

## Implementation Notes

- File: `src/app/components/agent_panel.rs` — add dropdown near the input `<form>`
- File: `src/app/ipc.rs` — add `list_models()` IPC wrapper
- Use Leptos signals for model list state (`RwSignal<Vec<ModelInfo>>`)
- Use `spawn_local` to fetch models on component mount
- Style: match existing stone/dark theme, compact height so it doesn't overwhelm the chat area

## Edge Cases

- Provider changes in settings → model list should refresh on next panel mount
- Empty model list → show "No models available" message
- Long model names → truncate with ellipsis in dropdown