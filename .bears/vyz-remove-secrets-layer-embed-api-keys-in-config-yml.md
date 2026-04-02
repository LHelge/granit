---
id: vyz
title: Remove secrets layer, embed API keys in config.yml
status: done
priority: P1
created: 2026-03-31T21:50:25.337812487Z
updated: 2026-04-02T12:56:28.106520797Z
tags:
- backend
depends_on:
- 7q3
parent: c56
---

## Summary

Remove the `secrets.env` layer entirely. API keys now live inside `ProviderConfig` variants in `config.yml`. Update the backend config module to drop secrets loading, the `Secrets` type, and all related infrastructure.

## Acceptance Criteria

- [ ] Delete `src-tauri/src/config/secrets.rs`
- [ ] Remove `Secrets` from `AppState` in `lib.rs`
- [ ] Remove `load_secrets()`, `ensure_cave()` gitignore entry for `secrets.env`, and `dotenvy` usage
- [ ] Remove `get_secret` and `set_secret` Tauri commands
- [ ] Remove `dotenvy` from `src-tauri/Cargo.toml` dependencies
- [ ] Remove `InvalidSecret` and `EnvFile` error variants from `ConfigError`
- [ ] Remove `validate_secret_value()` function
- [ ] Update `RawConfig` to use the new `AgentConfig` with `providers: Vec<ProviderConfig>` (Option-wrapped for merge layer)
- [ ] Update `MergeRaw` implementation for the new provider structure
- [ ] `save_config` command no longer handles secrets — just writes config
- [ ] Config YAML includes API keys inline
- [ ] `cargo test -p granit` passes

## Implementation Notes

- Files: `src-tauri/src/config/mod.rs`, `src-tauri/src/config/secrets.rs` (delete), `src-tauri/src/config/error.rs`, `src-tauri/src/lib.rs`
- The merge logic for `providers` can be simpler: cave config's `providers` entirely replaces global if present (no per-element merge)
- `save_global()` already writes the full config — it will now include API keys
- **Security note**: `config.yml` containing API keys should NOT be committed. The cave `.granit/config.yml` is already gitignored. The global config dir is outside any repo.