# Granit

Granit is a minimal, opinionated desktop note-taking app. It manages a local markdown "cave" with a rendered reader, an explorer sidebar, and an integrated AI agent.

Built for personal use. No plugins, no sync, no bloat.

## Current Features

- **Cave-based storage** — Any directory can be opened as a cave, and recently opened caves are tracked in global config.
- **Nested folders** — Notes can live in subdirectories, with a tree view, drag-and-drop moves, inline rename, and context-menu actions for notes and folders.
- **Rendered markdown reader** — Notes render to HTML via `pulldown-cmark`, with YAML frontmatter for tags, timestamps, and optional note icons.
- **Wiki-links** — `[[note]]` and `[[note|label]]` links resolve by filename across the cave. Broken links are styled separately so the frontend can handle them distinctly.
- **Explorer tabs** — The left pane has tree, full-text search, and todo views.
- **Todo support** — Task list checkboxes are parsed from notes, grouped in the todo tab, and can be toggled from the UI.
- **Daily notes** — Open or create today’s note in a configurable daily-note folder.
- **AI agent** — Streaming chat UI backed by `rig-core`, with provider/model selection and tools for note, folder, todo, search, and web operations.
- **Theme and font settings** — DaisyUI themes, Catppuccin variants, and per-surface font settings are configurable from the app.
- **Global config** — Settings are stored in `~/.config/granit/config.yml`.

## Cave Rules

- **Filenames are globally unique across the entire cave.** Two notes in different subfolders cannot share the same filename.
- **The filename is the note's identity and title.** Frontmatter does not override the displayed title.
- **Slugs are filename stems** with no extension or path. IPC calls and wiki-link resolution use slugs.
- **Hidden directories and `.granit/`** are excluded from cave scans.

## Current Config Model

Configuration is currently **global-only**.

```text
~/.config/granit/
  config.yml
```

- The app persists recent caves, sidebar widths/visibility, theme, font settings, daily-note folder, and agent/provider settings in `config.yml`.
- `active_cave` is runtime-only state and is not persisted.

## Deferred Features

- **Backlinks panel**
- **File watching / external change reload**
- **Obsidian-style live preview editor**
- **Sync**

## Tech Stack

| Layer | Tech |
|-------|------|
| Backend | Tauri 2 (Rust) |
| Frontend | Leptos 0.8 (Rust → WASM, CSR) |
| Build | Trunk |
| Styling | Tailwind CSS 4 + DaisyUI 5 |
| Markdown | `pulldown-cmark` |
| AI | `rig-core` |
| Errors | `thiserror` |
| Config | `serde_yml`, `dirs` |

## Development

### Prerequisites

- **Rust** (stable) with the WASM target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- **Tauri CLI** and native system dependencies:
  ```sh
  cargo install tauri-cli --locked
  ```
- **[Trunk](https://trunkrs.dev/)**:
  ```sh
  cargo install trunk --locked
  ```
- **[wasm-pack](https://rustwasm.github.io/wasm-pack/)** for frontend tests:
  ```sh
  cargo install wasm-pack --locked
  ```
- **[Tailwind CSS](https://tailwindcss.com/blog/standalone-cli)** standalone CLI available as `tailwindcss` on your `PATH`.

### Upgrading DaisyUI

`daisyui.mjs` and `daisyui-theme.mjs` are vendored in the repo. To upgrade:

```sh
DAISY_VERSION=v5.x.y
curl -sLO https://github.com/saadeghi/daisyui/releases/download/${DAISY_VERSION}/daisyui.mjs
curl -sLO https://github.com/saadeghi/daisyui/releases/download/${DAISY_VERSION}/daisyui-theme.mjs
```

Then commit both files.

### Build, Test, and Run

```sh
cd src-tauri && cargo tauri dev     # Full app (launches Trunk + Tauri)
trunk serve                         # Frontend only (port 1420)
cargo test -p granit                # Backend tests
cargo test -p granit-types          # Shared types tests
wasm-pack test --headless --firefox # Frontend WASM tests
cargo fmt --all
cargo clippy --all-targets --all-features
```

### Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Architecture

All persisted app state lives in the Tauri backend. The frontend is a thin Leptos view layer that talks to backend commands over IPC.

```text
src/
  app/
    agent/      # chat UI, streaming, provider/model selectors
    editor/     # writer, reader, frontmatter, smart text editing
    explorer/   # cave selector, tree view, search, todo
    settings/   # modal sections for agent/fonts/notes/theme
    components/ # shared UI helpers and icons
src-tauri/src/
  agent/        # rig-core integration and tool definitions
  cave/         # cave scanning and note/folder operations
  config/       # global config load/save
  markdown/     # frontmatter parsing and HTML rendering
  lib.rs        # Tauri command wiring and app state
```

See [.github/copilot-instructions.md](.github/copilot-instructions.md) for the internal architecture and coding conventions used in this repository.
