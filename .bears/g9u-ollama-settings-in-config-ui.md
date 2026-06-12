---
id: g9u
title: Ollama settings in config UI
status: done
priority: P2
created: 2026-03-27T21:34:29.952301723Z
updated: 2026-03-28T23:19:54.126189296Z
tags:
- frontend
- backend
depends_on:
- nbp
parent: ann
---

## Summary

Update the settings modal to expose Ollama-specific configuration: base URL, model name. Update `AgentConfig` defaults and ensure the config system persists these settings.

## Acceptance Criteria

- [ ] Settings modal shows Ollama base URL field (default: `http://localhost:11434`)
- [ ] Settings modal shows model name field (default: `llama3.2`)
- [ ] Changes are persisted to the global config via `save_config`
- [ ] Agent rebuilds with new config when settings are saved (or on next message)
- [ ] Provider field hidden or fixed to "ollama" for this epic (multi-provider comes later)

## Implementation Notes

- Files: `src/app/components/settings_modal.rs`, `granit-types/src/lib.rs`, `src-tauri/src/config/mod.rs`
- `AgentConfig` already has `provider` and `model` — just add `base_url: Option<String>`
- The settings modal already writes agent settings — extend the form

## Testing

- Manual: change model in settings, send a message, verify new model is used