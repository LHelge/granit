---
id: 6hv
title: Multi-provider LLM support (Ollama + Anthropic)
type: epic
status: open
priority: P2
created: 2026-03-27T21:41:08.222004859Z
updated: 2026-03-27T21:41:08.222004859Z
---

## Scope

Refactor the agent module to support multiple LLM providers behind a unified abstraction. Add Anthropic as the second provider alongside Ollama. Include settings UI for selecting the provider and managing API keys securely.

## Current State (after agent epic `ann`)

- Agent module hardcoded to Ollama provider
- `AgentConfig` has `provider`, `model`, `base_url` fields
- Secrets system exists (`secrets.env` with `dotenvy`, global + cave layers)
- Settings modal has Ollama-specific fields

## Out of Scope

- Additional providers beyond Ollama and Anthropic (later)
- Per-cave provider overrides (config layering already supports this, but no UI)

## Acceptance Criteria

- [ ] Provider abstraction: agent module can build an agent from any supported provider
- [ ] Ollama and Anthropic both work end-to-end (send message, streaming response)
- [ ] Settings UI: provider selector (dropdown), provider-specific fields (base URL for Ollama, API key for Anthropic)
- [ ] API keys stored in `secrets.env`, never in `config.yml`
- [ ] Switching providers in settings rebuilds the agent on next message
- [ ] Adding a new provider in the future requires minimal code (one match arm + config fields)