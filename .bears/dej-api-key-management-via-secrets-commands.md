---
id: dej
title: API key management via secrets commands
status: open
priority: P1
created: 2026-03-27T21:41:47.514900253Z
updated: 2026-03-27T21:41:47.514900253Z
tags:
- backend
parent: 6hv
---

## Summary

Add Tauri commands to read and write API keys via the secrets system. Keys are stored in `secrets.env` files, never in `config.yml`.

## Acceptance Criteria

- [ ] `get_secret(key: &str) -> Option<String>` Tauri command — reads from layered secrets (global ← cave)
- [ ] `set_secret(key: &str, value: &str)` Tauri command — writes to global `secrets.env`
- [ ] `ANTHROPIC_API_KEY` is the key name for the Anthropic provider
- [ ] Secrets are loaded on app start and refreshed on cave open
- [ ] Frontend IPC wrappers for both commands

## Implementation Notes

- Files: `src-tauri/src/config/secrets.rs`, `src-tauri/src/lib.rs`, `src/app/ipc.rs`
- The secrets system already exists (`load_secrets`, `Secrets` struct) — this adds Tauri command wrappers
- Writing `secrets.env`: append or update the key in the file (simple key=value format)
- Be careful with file locking / concurrent writes (unlikely for a single-user app but good to handle)

## Security

- Never log or serialize API key values
- `secrets.env` must be in `.gitignore` (already ensured by `ensure_cave_gitignore`)

## Testing

- Unit test: write then read a secret key
- Unit test: cave secret overrides global secret
- `cargo fmt && cargo clippy && cargo test`