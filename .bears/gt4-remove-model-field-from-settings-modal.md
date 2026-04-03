---
id: gt4
title: Remove model field from settings modal
status: done
priority: P2
created: 2026-03-29T00:06:59.685104740Z
updated: 2026-04-03T23:36:12.641210011Z
tags:
- frontend
depends_on:
- mwp
parent: xd5
---

## Summary

Remove the model text input from the settings modal since model selection now lives in the agent panel dropdown. Keep provider, base URL, and API key fields in settings.

## Acceptance Criteria

- [ ] Model field removed from settings modal UI
- [ ] Provider, base URL, and API key fields remain and work as before
- [ ] Changing provider in settings still resets the model to the provider's default
- [ ] No compile warnings or dead code from the removal

## Implementation Notes

- File: `src/app/components/settings_modal.rs`
- The `AgentConfig.model` field stays in the struct (still used by backend) — only the UI input is removed
- When provider changes, set model to provider default (e.g., "qwen3.5:9b" for Ollama, "claude-sonnet-4-20250514" for Anthropic)