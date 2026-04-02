---
id: yw5
title: Update Agent initialization for ProviderConfig enum
status: done
priority: P1
created: 2026-03-31T21:50:35.286909569Z
updated: 2026-04-02T12:56:28.112998255Z
tags:
- backend
depends_on:
- vyz
parent: c56
---

## Summary

Update `Agent` to initialize from a single `ProviderConfig` enum variant instead of the old flat `AgentConfig` + `Secrets`. Support selecting from the providers vector by index.

## Acceptance Criteria

- [ ] `Agent::from_provider_config(provider: &ProviderConfig, max_history: usize)` replaces `from_config(config, secrets)`
- [ ] `build_ollama`, `build_anthropic`, `build_mistral` take their respective variant fields directly (no secrets lookup)
- [ ] Remove `use crate::config::Secrets` from agent module
- [ ] `reset_agent()` in `lib.rs` rebuilds from `config.agent.providers[config.agent.selected_provider]`
- [ ] `send_message` still snapshots and streams correctly
- [ ] `cargo test -p granit` passes

## Implementation Notes

- File: `src-tauri/src/agent/mod.rs`, `src-tauri/src/agent/error.rs` (remove `MissingApiKey` if no longer needed — the key is guaranteed present by the enum variant), `src-tauri/src/lib.rs`
- Pattern match on `ProviderConfig` variant to dispatch to the correct builder
- `UnknownProvider` error variant can also be removed since exhaustive match on enum handles all cases