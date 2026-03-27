---
id: e9d
title: Provider abstraction in agent module
status: open
priority: P1
created: 2026-03-27T21:41:24.257081459Z
updated: 2026-03-27T21:41:24.257081459Z
tags:
- backend
parent: 6hv
---

## Summary

Refactor the agent builder to use a provider abstraction so the same chat interface works regardless of which LLM backend is configured. Extract the Ollama-specific code behind this abstraction.

## Acceptance Criteria

- [ ] Agent builder takes `AgentConfig` and returns a provider-agnostic agent (rig-core's `Agent` type is already generic)
- [ ] Match on `config.provider` to construct the correct rig-core client (`"ollama"` → `ollama::Client`, `"anthropic"` → `anthropic::Client`)
- [ ] Ollama path continues to work exactly as before (no regression)
- [ ] Unknown provider string returns a clear error
- [ ] Agent rebuild function: given new config, tear down old agent and build a new one

## Implementation Notes

- Files: `src-tauri/src/agent/mod.rs`
- rig-core's `Agent` is generic over the completion model — you may need a trait object or enum dispatch to hold different provider agents in the same `AgentState`
- Consider `Box<dyn StreamingPrompt>` or an enum like `enum ProviderAgent { Ollama(...), Anthropic(...) }`
- Keep it simple: enum dispatch is fine for two providers, no need for a plugin registry

## Testing

- Existing tests still pass with Ollama
- Unit test: building an agent with provider `"ollama"` succeeds
- Unit test: building an agent with unknown provider returns error
- `cargo fmt && cargo clippy && cargo test`