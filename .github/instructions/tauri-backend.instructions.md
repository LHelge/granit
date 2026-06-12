---
applyTo: "src-tauri/**"
---

# Tauri Backend

Instructions for `src-tauri/`.

## Core Rules

- Keep Tauri command handlers thin. Put logic in modules under `commands/`, `cave/`, `markdown/`, and `agent/`.
- Use typed `thiserror` enums that serialize cleanly across IPC. Do not use `anyhow`.
- Prefer direct module tests over testing through the Tauri command layer.
- Keep `lib.rs` focused on builder setup, plugin registration, and command wiring.

## Backend State

- The backend owns app state.
- Current shared state includes `AppConfig`, the active cave handle, and the cached agent.

## Config Reality

- `<cave>/.granit/config.yml` is the persisted config source of truth.
- `<cave>/.granit/templates/` stores cave-local templates.
- `active_cave` is runtime-only and must not be serialized into cave YAML.
- The last open cave path is persisted separately via `tauri-plugin-store`.
