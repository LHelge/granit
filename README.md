# Granit

A minimal, opinionated desktop note-taking app. Granit manages a **cave** — a directory of markdown files — with an integrated AI agent.

Built for personal use. No plugins, no sync, no bloat.

## Current Features

- **Cave-based storage** — Any directory is a cave. Open, switch, and track recently opened caves.
- **Note CRUD** — Create, read, rename, and delete markdown notes. Filenames are the source of truth for note identity.
- **Nested folders** — Notes can live in any subdirectory of the cave. The sidebar shows the full folder hierarchy with collapsible folder nodes. See [Nested folder rules](#nested-folder-rules) below.
- **Edit / Preview toggle** — Raw markdown editing with plaintext preview.
- **Global settings** — Configurable AI agent provider and model saved to `~/.config/granit/config.yml`.
- **Layered config** — Global config can be overridden per-cave via `<cave>/.granit/config.yml` (UI editing not yet exposed).

## Nested Folder Rules

- **Filenames are globally unique across the entire cave.** Two notes in different subfolders cannot share the same filename (e.g., `projects/foo.md` and `archive/foo.md` cannot coexist). The cave will log a warning and skip duplicates on scan.
- **The filename is the note's identity and title.** Frontmatter `title` fields and markdown headings do not override the displayed title. `projects/meeting.md` is always displayed as `meeting`.
- **Slugs are filename stems** (no extension, no path). All lookups and IPC calls use the slug. The `relative_path` field is available for display and tree building only.
- **Hidden directories and `.granit/`** are excluded from the scan.
- **Cave operations accept an optional folder path.** `create_note(name, folder)` places the new note in the specified subfolder. `create_folder(path)` creates a (possibly nested) directory.

## Planned / Not Yet Implemented

- **Rendered markdown preview** — Preview currently shows raw text. `pulldown-cmark` HTML rendering is planned.
- **AI Agent** — Side panel UI is scaffolded but not yet connected. `rig-core` integration, cave CRUD tools, and RAG over notes are roadmap items.
- **Create note in folder from UI** — The backend supports `create_note(name, folder)` but the UI always creates notes at the cave root for now.
- **Wiki-links** — `[[note-name]]` link resolution is planned but not yet implemented.
- **Full-text search** — Not yet implemented.
- **Backlinks panel** — Not yet implemented.

## Tech Stack

| Layer | Tech |
|-------|------|
| Backend | Tauri 2 (Rust) |
| Frontend | Leptos 0.8 (Rust → WASM, CSR) |
| Build | Trunk |
| Styling | Tailwind CSS |
| Errors | `thiserror` |
| Config | `serde_yml`, `dirs`, `dotenvy` |

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
### Upgrading DaisyUI

`daisyui.mjs` and `daisyui-theme.mjs` are vendored in the repo (v5.5.19). To upgrade:

```sh
DAISY_VERSION=v5.x.y
curl -sLO https://github.com/saadeghi/daisyui/releases/download/${DAISY_VERSION}/daisyui.mjs
curl -sLO https://github.com/saadeghi/daisyui/releases/download/${DAISY_VERSION}/daisyui-theme.mjs
```

Then commit both files.

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
