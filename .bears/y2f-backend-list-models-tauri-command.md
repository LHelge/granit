---
id: y2f
title: 'Backend: list_models Tauri command'
status: done
priority: P1
created: 2026-03-29T00:06:40.109066543Z
updated: 2026-04-02T13:21:44.932750218Z
tags:
- backend
depends_on:
- ruw
parent: xd5
---

## Summary

Add a `list_models` Tauri command that queries the current provider's API for available models. Uses direct HTTP calls since rig-core 0.33.0 doesn't implement `ModelListingClient` for either provider.

## Acceptance Criteria

- [ ] `list_models` Tauri command returns `Result<Vec<ModelInfo>, AgentError>`
- [ ] Ollama: `GET {base_url}/api/tags` — parse `models[].name` into `ModelInfo`
- [ ] Anthropic: `GET https://api.anthropic.com/v1/models` with `x-api-key` header
- [ ] Returns `AgentError` when provider is unreachable or returns an error
- [ ] Unit tests for response parsing

## Implementation Notes

- File: `src-tauri/src/agent/mod.rs` (or a new `src-tauri/src/agent/models.rs` submodule)
- File: `src-tauri/src/lib.rs` (register the command)
- Add `reqwest` as a direct dependency with `json` feature
- Ollama response shape: `{"models": [{"name": "qwen3.5:9b", "model": "qwen3.5:9b", ...}]}`
- Anthropic response shape: `{"data": [{"id": "claude-sonnet-4-20250514", "display_name": "Claude Sonnet 4", ...}]}`
- Read provider config and secrets from `AppState` to build the request

## Edge Cases

- Provider unreachable (timeout, connection refused) → friendly error
- Missing API key for Anthropic → return `MissingApiKey` error
- Empty model list → return empty vec (not an error)