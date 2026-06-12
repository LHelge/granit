---
id: ann
title: Basic agent chat with Ollama
type: epic
status: done
priority: P1
created: 2026-03-27T21:33:13.847854674Z
updated: 2026-03-28T23:19:57.987797241Z
---

## Scope

Add a basic AI chat agent using rig-core with Ollama as the provider. No tools, no RAG — just a plain conversational chat with streaming responses displayed in the existing agent panel.

## Current State

- `AgentPanel` component exists with input box and placeholder message area
- `AgentConfig` exists in config system (provider + model fields, defaults to openai/gpt-4o)
- No `rig-core` dependency, no `agent/` module in backend
- No streaming or Tauri event infrastructure

## Out of Scope (later epics)

- Cave tools (read/create/update/delete notes)
- RAG / vector DB over cave contents
- Multiple provider support (OpenAI, Anthropic, etc.) — Ollama only for now

## Acceptance Criteria

- [ ] Backend agent module (`src-tauri/src/agent/`) with Ollama client setup
- [ ] User can send a message and receive a streaming response in the agent panel
- [ ] Conversation history is maintained within the session
- [ ] Streaming tokens appear progressively in the UI (not all-at-once)
- [ ] Agent responses are rendered with markdown formatting
- [ ] Config: Ollama base URL and model name configurable in settings
- [ ] Graceful error handling: Ollama not running, model not found, etc.
- [ ] Default model: `qwen3.5:9b`