# Granit

A minimal, opinionated desktop note-taking app. Granit manages a **cave** — a directory of markdown files — with an integrated AI agent.

Built for personal use. No plugins, no sync, no bloat.

## Features

- **Cave-based storage** — Any directory is a cave. Nested folders for organization. Wiki-style `[[links]]` resolved by filename.
- **Markdown-first** — YAML frontmatter for metadata. `pulldown-cmark` renders on the backend.
- **Edit / Read toggle** — Raw markdown editing with rendered HTML preview.
- **AI Agent** — Side panel chat powered by `rig-core`. CRUD tools for cave operations. In-memory vector DB for RAG. Configurable LLM provider.
- **Multi-cave** — One cave open at a time, switchable via recent caves list.

## Tech Stack

| Layer | Tech |
|-------|------|
| Backend | Tauri 2 (Rust) |
| Frontend | Leptos 0.8 (Rust → WASM, CSR) |
| Build | Trunk |
| Styling | Tailwind CSS |
| Markdown | `pulldown-cmark` |
| AI | `rig-core` |
| Errors | `thiserror` |

## Development

### Prerequisites

- Rust (stable)
- [Tauri CLI](https://tauri.app/start/create-project/#prerequisites)
- [Trunk](https://trunkrs.dev/)

### Build & Run

```sh
cd src-tauri && cargo tauri dev    # Full app (launches Trunk + Tauri)
trunk serve                        # Frontend only (port 1420)
cargo test -p granit               # Backend unit tests
```

### Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Architecture

All state lives in the backend. The frontend is a thin view layer that calls Tauri commands via IPC.

```
src/            — Leptos frontend (WASM)
src-tauri/src/  — Tauri backend (native Rust)
```

See [.github/copilot-instructions.md](.github/copilot-instructions.md) for detailed architecture and conventions.
