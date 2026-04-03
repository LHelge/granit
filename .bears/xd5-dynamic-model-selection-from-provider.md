---
id: xd5
title: Dynamic model selection from provider
type: epic
status: done
priority: P1
created: 2026-03-29T00:06:21.075709713Z
updated: 2026-04-03T23:36:15.623606563Z
---

## Scope

Replace the static model field in config with a dynamic model selector in the agent panel. Query each provider's API for available models and let the user pick from a dropdown near the chat input.

## Key Design Decisions

- **Model stays in config** as "last used model" so the choice persists between sessions
- **Model selector lives in the agent panel** (near chat input), not in settings
- **Direct HTTP calls** to provider APIs since rig-core 0.33.0's `ModelListingClient` trait is not implemented for Ollama or Anthropic
- **Ollama**: `GET {base_url}/api/tags` → `{"models": [{"name": "...", ...}]}`
- **Anthropic**: `GET https://api.anthropic.com/v1/models` with `x-api-key` header
- **reqwest** for HTTP calls (already a transitive dep via rig-core)

## Acceptance Criteria

- [ ] `list_models` Tauri command returns available models for the configured provider
- [ ] Agent panel shows a model selector dropdown near the chat input
- [ ] Selecting a model updates config, persists it, and resets the agent
- [ ] Model field removed from settings modal (provider + base URL + API key remain)
- [ ] Graceful error handling when provider is unreachable