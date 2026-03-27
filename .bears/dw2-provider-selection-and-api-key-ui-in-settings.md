---
id: dw2
title: Provider selection and API key UI in settings
status: open
priority: P1
created: 2026-03-27T21:42:07.930289460Z
updated: 2026-03-27T21:42:07.930289460Z
tags:
- frontend
depends_on:
- e9d
- dej
parent: 6hv
---

## Summary

Redesign the agent section of the settings modal to support provider selection and provider-specific configuration fields (base URL for Ollama, API key for Anthropic).

## Acceptance Criteria

- [ ] Provider selector: dropdown/select with options "ollama" and "anthropic"
- [ ] Conditional fields based on selected provider:
  - Ollama: base URL (text, default `http://localhost:11434`), model name
  - Anthropic: API key (password input, masked), model name (default `claude-sonnet-4-20250514`)
- [ ] API key field reads existing key from secrets on modal open (masked display)
- [ ] On save: provider + model written to `config.yml`, API key written to `secrets.env` via the secrets command
- [ ] Switching provider clears/resets provider-specific fields to defaults
- [ ] API key field shows "configured" / "not set" indicator without revealing the key

## Implementation Notes

- Files: `src/app/components/settings_modal.rs`, `src/app/ipc.rs`
- Use a `<select>` element for provider choice
- Show/hide fields reactively based on selected provider using Leptos `Show` component
- API key: use `<input type="password">` — read via `ipc::get_secret("ANTHROPIC_API_KEY")` on open, write via `ipc::set_secret(...)` on save
- Consider showing just a "Key is set" / "No key configured" status instead of loading the actual key into the form

## Testing

- Manual: switch between providers, verify correct fields appear
- Manual: set Anthropic API key, close and reopen settings, verify "configured" indicator