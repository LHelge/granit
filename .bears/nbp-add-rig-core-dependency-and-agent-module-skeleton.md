---
id: nbp
title: Add rig-core dependency and agent module skeleton
status: open
priority: P1
created: 2026-03-27T21:33:26.270680920Z
updated: 2026-03-27T21:33:26.270680920Z
tags:
- backend
parent: ann
---

## Summary

Add `rig-core` and `tokio` to the backend, create the `src-tauri/src/agent/` module with an Ollama client builder. Update `AgentConfig` defaults to `ollama`/`llama3.2`.

## Acceptance Criteria

- [ ] `rig-core` and `tokio` (with `rt-multi-thread`, `macros` features) added via `cargo add`
- [ ] `src-tauri/src/agent/mod.rs` created with a function to build an Ollama-backed agent from config
- [ ] Agent builder reads Ollama base URL from config (default `http://localhost:11434`)
- [ ] `AgentConfig` updated: add `base_url: Option<String>` field, default provider changed to `"ollama"`, default model to `"llama3.2"`
- [ ] Module declared in `src-tauri/src/lib.rs`
- [ ] Basic unit test: agent struct builds without error (no actual LLM call)

## Implementation Notes

- Files: create `src-tauri/src/agent/mod.rs`, update `src-tauri/src/lib.rs`, update `granit-types/src/lib.rs`
- Ollama client: `ollama::Client::new(Nothing)` for default URL, or use builder for custom base URL
- Config settings modal already exists — it writes `provider` and `model` fields

## Testing

- `cargo fmt && cargo clippy && cargo test` must pass