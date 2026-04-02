---
id: hq2
title: 'Backend: list_providers, select_provider, list_models commands'
status: done
priority: P1
created: 2026-03-31T21:50:51.113842920Z
updated: 2026-04-02T13:21:44.924875771Z
tags:
- backend
depends_on:
- yw5
parent: c56
---

## Summary

Add Tauri commands for the frontend to list configured providers, select a provider, list available models from the selected provider's API, and select a model for the active agent session.

## Acceptance Criteria

- [ ] `list_providers` command → returns `Vec<ProviderInfo>` (index, display_name, provider_type) from config
- [ ] `select_provider(index: usize)` → updates `config.agent.selected_provider`, persists config, resets agent
- [ ] `list_models` command → queries the selected provider's API and returns `Vec<ModelInfo>`
- [ ] `select_model(model_id: String)` → rebuilds agent with specified model, persists last-used model in config
- [ ] Graceful error when provider is unreachable (return error, don't panic)
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/lib.rs` (new commands + handler registration), possibly a helper in `src-tauri/src/agent/mod.rs` for model listing
- **Model listing by provider:**
  - Ollama: `GET {base_url}/api/tags` → parse JSON `{"models": [{"name": "..."}]}`
  - Anthropic: `GET https://api.anthropic.com/v1/models` with `x-api-key` and `anthropic-version` headers
  - Mistral: `GET https://api.mistral.ai/v1/models` with `Authorization: Bearer {key}`
  - Check if rig-core clients expose a `list_models()` method first — if so, prefer that over raw HTTP
- Use `reqwest` (already a transitive dep) if rig doesn't provide model listing
- `ProviderInfo` can be a lightweight type in `granit-types` or just return provider display names with indices
- `select_model` should also persist the chosen model to config so it's remembered across restarts — may need a `last_model: Option<String>` field per provider or in `AgentConfig`