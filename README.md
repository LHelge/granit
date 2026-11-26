# Granit

Granit is a minimal, opinionated desktop note-taking app. It manages a local markdown "cave" with a rendered reader, an explorer sidebar, and an integrated AI agent.

Built for personal use. No plugins, no sync, no bloat.

The documentation lives at <https://granit.lhelge.se>.

## Installation

Prebuilt bundles are published on the [GitHub releases page](https://github.com/LHelge/granit/releases). Quick install:

```sh
# macOS (Apple Silicon) — Homebrew
brew install --cask lhelge/tap/granit

# Linux (x86_64) — install script
curl -fsSL https://granit.lhelge.se/static/install.sh | sh

# Windows — download the .exe or .msi from the releases page
```

See the [installation guide](https://granit.lhelge.se/wiki/installation/) for per-platform notes, automatic updates, and building from source.

## Features

- **Local markdown caves** — any directory of plain `.md` files, with nested folders, cave-local settings, and globally unique note names.
- **Wiki-links, heading anchors, and backlinks** — `[[note]]` links resolve by filename across the cave; renaming a note rewrites the links to it.
- **Reader and editor** — backend-rendered markdown (alerts, tables, footnotes, Mermaid diagrams, images) and a CodeMirror editor with autosave.
- **Explorer** — tree, search, todos, tags, favorites, templates, and agent files, with a daily-note calendar.
- **Templates and daily notes** — Tera note templates and a configurable daily-note folder.
- **AI agent** — streaming chat with Ollama, Anthropic, Mistral, or any OpenAI-compatible endpoint; Ask mode with local RAG, Agent mode with tools that edit the cave; an editable system prompt, skills, and tasks.
- **Presentations** — present any note as slides in a separate window, styled by per-cave CSS templates.
- **Encrypted backups** — snapshot a cave to a [self-hosted server](backend/README.md), encrypted locally with a passphrase.
- **Themes and automatic updates** — every DaisyUI theme plus Catppuccin and custom ones; silent updates from GitHub releases.

## Development

Prerequisites: Node.js + npm, stable Rust with the `wasm32-unknown-unknown` target, and these tools:

```sh
rustup target add wasm32-unknown-unknown
cargo install tauri-cli trunk wasm-pack --locked
npm ci
```

Then:

```sh
npm run build                     # CSS + CodeMirror/Mermaid bundles into build/
cd src-tauri && cargo tauri dev   # full app (launches Trunk + Tauri)
```

[CLAUDE.md](CLAUDE.md) describes the workspace layout and architecture, and lists every lint and test command CI runs. The backup server has its own [README](backend/README.md).

### Documentation site

The site is built from `docs/` with [aphid](https://github.com/LHelge/aphid):

```sh
cargo install aphid --locked   # one-time
cd docs
aphid serve                    # live-reload preview on :3000
aphid build                    # build + validate wiki-links
```

The `docs` job of the [release workflow](.github/workflows/release.yml) builds the site into an nginx image and deploys it to hoster ([docs/docs.stack.toml](docs/docs.stack.toml)) on every release tag.

### Releases and auto-updates

Tagging `v*.*.*` runs the release workflow: it generates the changelog from commit messages with `git-cliff`, builds signed bundles via `tauri-action` (including the updater artifacts and the `latest.json` manifest, whose notes become the in-app release notes), publishes the backup server image to GHCR, and publishes the GitHub release. The updater needs two repository secrets, `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, matching the public key in `src-tauri/tauri.conf.json`.

- On Linux only AppImage installs self-update; `.deb`/`.rpm` users update manually.
- The update endpoint resolves `releases/latest`, so an update becomes visible only once the release is published (the workflow's final step).
- Dev builds never run the startup check; the manual check in the About dialog still works.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
