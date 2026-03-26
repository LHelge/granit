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

- **Rust** (stable) with the WASM compile target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- **Tauri CLI** and system dependencies — follow the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for your platform (on Linux this includes `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc.):
  ```sh
  cargo install tauri-cli --locked
  ```
- **[Trunk](https://trunkrs.dev/)** — WASM build tool for the Leptos frontend:
  ```sh
  cargo install trunk --locked
  ```
- **[Tailwind CSS](https://tailwindcss.com/blog/standalone-cli)** standalone CLI — must be available as `tailwindcss` on your `PATH` (used by Trunk's pre-build hook):
  ```sh
  # Example for Linux x64:
  curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64
  chmod +x tailwindcss-linux-x64
  sudo mv tailwindcss-linux-x64 /usr/local/bin/tailwindcss
  ```

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
