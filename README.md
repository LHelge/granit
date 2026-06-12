# Granit

Granit is a minimal, opinionated desktop note-taking app. It manages a local markdown "cave" with a rendered reader, an explorer sidebar, and an integrated AI agent.

Built for personal use. No plugins, no sync, no bloat.

## Current Features

- **Cave-based storage** — Any directory can be opened as a cave, and each cave stores its own settings in `.granit/config.yml`.
- **Local templates** — Each cave can also store reusable note templates in `.granit/templates/`.
- **Nested folders** — Notes can live in subdirectories, with a tree view, drag-and-drop moves, inline rename, and context-menu actions for notes and folders.
- **Rendered markdown reader** — Notes render to HTML via `pulldown-cmark`, with YAML frontmatter for tags, timestamps, optional note icons, and an optional `favorite` flag.
- **Wiki-links** — `[[note]]` and `[[note|label]]` links resolve by filename across the cave. Broken links are styled separately so the frontend can handle them distinctly.
- **Explorer tabs** — The left pane includes tree, full-text search, todo, tag, favorites, and template views, plus a compact daily-note calendar strip.
- **Todo support** — Task list checkboxes are parsed from notes, grouped in the todo tab, and can be toggled from the UI.
- **Tags and favorites** — Tags are indexed cave-wide from frontmatter, and notes can be marked as favorites from frontmatter.
- **Daily notes** — Open or create today’s note in a configurable daily-note folder, optionally seeded from a template.
- **AI agent** — Streaming chat UI backed by `rig-core`, with provider/model selection and tools for note, folder, template, todo, search, and web operations.
- **Theme and font settings** — DaisyUI themes, Catppuccin variants, and per-surface font settings are configurable from the app.
- **Active cave restore** — The last open cave is restored from app state without keeping a recent-caves list.
- **Automatic updates** — On startup the app silently downloads and installs new versions from GitHub releases, offers a restart, and shows the release notes after the update. A manual check lives in the About dialog.

## Cave Rules

- **Filenames are globally unique across the entire cave.** Two notes in different subfolders cannot share the same filename.
- **The filename is the note's identity and title.** Frontmatter does not override the displayed title.
- **Slugs are filename stems** with no extension or path. IPC calls and wiki-link resolution use slugs.
- **Templates live outside the note tree** in `.granit/templates/` and use their own flat slug namespace.
- **Hidden directories and `.granit/`** are excluded from normal cave note scans.

## Current Config Model

Configuration is **cave-local**.

```text
<cave>/
  .granit/
    config.yml
    templates/
```

- Each cave persists its own sidebar widths/visibility, theme, font settings, daily-note folder, optional daily-note template slug, and agent/provider settings in `.granit/config.yml`.
- The active cave path is stored separately in Tauri store app state and used to restore the last open cave on startup.
- `active_cave` is runtime-only in IPC responses and is not serialized into cave YAML.

## Deferred Features

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
| Config | `serde_yml`, `tauri-plugin-store` |

## Development

### Prerequisites

- **Node.js + npm** for the frontend build pipeline:
  ```sh
  npm ci
  ```
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

The frontend build uses the npm-installed Tailwind CLI and esbuild. Generated assets are written to `build/`, and Trunk packages the app into `dist/` for Tauri builds.

### Build, Test, and Run

```sh
npm run build                        # Build CSS + CodeMirror bundle into build/
cd src-tauri && cargo tauri dev     # Full app (launches Trunk + Tauri)
trunk serve                         # Frontend only (port 1420)
cargo test -p granit                # Backend tests
cargo test -p granit-types          # Shared types tests
wasm-pack test --headless --firefox # Frontend WASM tests
cargo fmt --all
cargo clippy --all-targets --all-features
```

### Releases and auto-updates

Tagging `v*.*.*` runs the release workflow: it generates the changelog with `git-cliff`, builds signed bundles via `tauri-action` (including the updater artifacts and `latest.json` manifest, whose notes become the in-app release notes), and publishes the GitHub release. The updater needs two repository secrets, `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, matching the public key in `src-tauri/tauri.conf.json`.

Caveats:

- On Linux only AppImage installs self-update; `.deb`/`.rpm` users must update manually.
- The update endpoint resolves `releases/latest`, so updates become visible only once the release is published (the workflow's final step).
- Dev builds never run the startup check; the manual check in the About dialog still works.

### Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Architecture

All persisted app state lives in the Tauri backend. The frontend is a thin Leptos view layer that talks to backend commands over IPC.

```text
granit-types/
  src/          # shared IPC/config/document/agent types
js/
  editor.ts     # CodeMirror 6 wrapper bundled with esbuild
build/          # generated CSS + JS assets consumed by Trunk
src/
  app/
    agent/      # chat UI, streaming, provider/model selectors
    editor/     # writer, reader, frontmatter, smart text editing
    explorer/   # calendar, cave selector, tree view, search, todo, tags, favorites, templates
    settings/   # modal sections for agent/fonts/notes/theme
    components/ # shared UI helpers and icons
src-tauri/src/
  agent/        # rig-core integration and tool definitions
  cave/         # cave scanning and note/folder operations
  commands/     # Tauri command handlers, app state, active-cave persistence
  markdown/     # frontmatter parsing and HTML rendering
  lib.rs        # Tauri command wiring and app state
```

See [.github/copilot-instructions.md](.github/copilot-instructions.md) for the internal architecture and coding conventions used in this repository.
