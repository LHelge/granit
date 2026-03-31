---
id: 7q3
title: Define ProviderConfig enum and update AgentConfig in granit-types
status: open
priority: P1
created: 2026-03-31T21:50:10.547711304Z
updated: 2026-03-31T21:50:10.547711304Z
tags:
- core
parent: c56
---

## Summary

Replace the flat `AgentConfig` struct with a tagged `ProviderConfig` enum in `granit-types`. Each variant carries only the fields relevant to that provider. Keep agent-level settings (max_history) separate.

## Acceptance Criteria

- [ ] `ProviderConfig` enum with variants: `Ollama { base_url: Option<String> }`, `Anthropic { api_key: String }`, `Mistral { api_key: String, base_url: Option<String> }`
- [ ] Uses `#[serde(tag = "provider", rename_all = "lowercase")]` for clean YAML output
- [ ] Each variant has a `display_name()` method returning a user-friendly label (e.g. "Ollama (localhost:11434)" or "Anthropic")
- [ ] `AgentConfig` updated: `providers: Vec<ProviderConfig>`, `selected_provider: usize`, `max_history: usize` — remove old `provider`, `model`, `base_url` fields
- [ ] `ModelInfo` struct: `id: String`, `name: Option<String>`, `display_name()` method
- [ ] All types derive `Debug, Clone, Serialize, Deserialize`
- [ ] Default for `AgentConfig` includes one Ollama provider with default base_url
- [ ] `cargo test -p granit-types` passes

## Implementation Notes

- File: `granit-types/src/agent.rs` (modify), `granit-types/src/lib.rs` (re-export ModelInfo)
- Keep `ChatMessage` and `ChatRole` unchanged
- The `selected_provider` field indexes into `providers` vec — the frontend can use this to pre-select the dropdown
- Consider adding a `name` field to each ProviderConfig variant for disambiguation when multiple providers of the same type exist — or derive it from variant + distinguishing field

## YAML example after change

```yaml
agent:
  max_history: 100
  selected_provider: 0
  providers:
    - provider: ollama
      base_url: http://localhost:11434
    - provider: anthropic
      api_key: sk-ant-...
    - provider: mistral
      api_key: mist-...
```