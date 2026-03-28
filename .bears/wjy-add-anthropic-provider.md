---
id: wjy
title: Add Anthropic provider
status: done
priority: P1
created: 2026-03-27T21:41:34.912883009Z
updated: 2026-03-28T23:36:07.372895011Z
tags:
- backend
depends_on:
- e9d
parent: 6hv
---

## Summary

Add the Anthropic provider to the agent module. Wire it through the provider abstraction so it supports the same streaming chat flow as Ollama.

## Acceptance Criteria

- [ ] `anthropic::Client` constructed with API key from secrets
- [ ] Streaming chat works end-to-end with Anthropic (same Tauri events as Ollama)
- [ ] Model name configurable (default: `claude-sonnet-4-20250514`)
- [ ] Missing API key returns a clear error ("Anthropic API key not configured")
- [ ] Conversation history works the same as with Ollama

## Implementation Notes

- Files: `src-tauri/src/agent/mod.rs`
- Anthropic client: `anthropic::Client::new(&api_key)` — API key comes from `Secrets`
- Add a new match arm in the provider builder: `"anthropic"` → build Anthropic agent
- Anthropic doesn't need a `base_url` field (unlike Ollama)
- rig-core handles the Anthropic streaming protocol internally

## Testing

- Manual: configure Anthropic in settings, send a message, verify streaming works
- Unit test: agent builds with provider `"anthropic"` when API key is present
- Unit test: agent build fails gracefully when API key is missing