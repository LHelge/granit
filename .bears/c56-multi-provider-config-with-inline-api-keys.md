---
id: c56
title: Multi-provider config with inline API keys
type: epic
status: done
priority: P1
created: 2026-03-31T21:49:38.490463472Z
updated: 2026-04-03T23:36:15.628704105Z
---

## Scope

Restructure the LLM provider configuration to:

1. **Merge secrets into config** — store API keys directly in `config.yml` alongside other provider settings, eliminating the separate `secrets.env` layer.
2. **Provider as tagged enum** — each provider variant has its own fields (Ollama has `base_url`, Anthropic has `api_key`, Mistral has `api_key` + optional `base_url`, etc.).
3. **Multiple providers** — config holds a `Vec<ProviderConfig>` so users can configure e.g. two Mistral instances with different API keys, or one Anthropic + one Ollama.
4. **Provider + model selector in agent panel** — two dropdowns above the chat input: one to pick the active provider, one to pick the model (fetched dynamically from the provider's API).

## Supersedes

This epic supersedes `xd5` (Dynamic model selection from provider) which covered only the model dropdown. This epic is broader: it includes config restructuring, multi-provider support, and the model selector.

## Key Design Decisions

- API keys live in `config.yml` — no more `secrets.env` files or `dotenvy` loading.
- `ProviderConfig` is a `#[serde(tag = "provider")]` enum so YAML looks like:
  ```yaml
  providers:
    - provider: ollama
      base_url: http://localhost:11434
    - provider: anthropic
      api_key: sk-ant-...
    - provider: mistral
      api_key: ...
      base_url: https://custom.endpoint/v1  # optional
  ```
- The agent panel dropdowns drive provider/model selection at runtime.
- Model list is fetched from the provider API (not hardcoded).
- Selected provider index + last-used model persist in config.

## Acceptance Criteria

- [ ] `secrets.env` files and the secrets module are removed
- [ ] Provider config is a tagged enum serialized inline in `config.yml`
- [ ] Config supports multiple providers in a vector
- [ ] Agent panel has provider dropdown + model dropdown above chat input
- [ ] Models are fetched dynamically from the selected provider's API
- [ ] Switching provider/model resets the agent and persists the choice
- [ ] Settings UI supports adding, editing, and removing providers
- [ ] `cargo fmt && cargo clippy && cargo test` pass