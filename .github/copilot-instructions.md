# Granit — Copilot Instructions

Granit is a minimal, opinionated desktop note-taking app built for personal use. It manages a "cave" — a directory of markdown files — with an integrated AI agent.

## Tech Stack

- **Backend**: Tauri 2 (Rust) — single source of truth for all data and logic
- **Frontend**: Leptos 0.8 (Rust → WASM, CSR mode) compiled with Trunk
- **Styling**: Tailwind CSS (utility classes in `view!` macros)
- **Markdown**: `pulldown-cmark` in the backend — frontend receives rendered HTML
- **AI Agent**: `rig-core` with configurable LLM provider (OpenAI, Anthropic, etc.)
- **Error handling**: `thiserror` for typed error enums
- **Serialization**: `serde` + `serde_json` (backend), `serde-wasm-bindgen` (frontend IPC)

## Architecture

```
src/            — Leptos frontend (WASM)
  app.rs        — Root component and UI
  main.rs       — WASM entry point
src-tauri/src/  — Tauri backend (native Rust)
  lib.rs        — Tauri commands, app builder, plugin registration
  main.rs       — Desktop entry point
```

### Data Flow

All state lives in the backend. The frontend is a thin view layer.

1. Frontend calls `invoke("command_name", args)` via Tauri IPC
2. Backend processes the command (file I/O, markdown parsing, agent calls)
3. Backend returns serialized result
4. Frontend updates reactive signals with the response

Never duplicate logic between frontend and backend. If the frontend needs derived data, add a backend command.

### Cave Concept

A **cave** is any directory on disk selected via the native folder picker. It contains:
- Nested subdirectories (folders as organizational hierarchy)
- `.md` files with optional YAML frontmatter (title, tags, date)
- Wiki-style `[[links]]` resolved by **filename** (not path) across the entire cave

One cave is open at a time. The user can switch between recently opened caves.

### Configuration

Layered config with precedence: **defaults ← global ← cave**.

```
~/.config/granit/          (or platform equivalent via dirs::config_dir())
  config.yml               — Global settings (recent caves, default agent provider/model)
  secrets.env              — Global API keys (AGENT_API_KEY, etc.)

<cave>/.granit/
  config.yml               — Per-cave overrides (e.g., different model)
  secrets.env              — Per-cave API keys (overrides global)
```

- Config structs use `Option<T>` fields for the cave layer — unset fields fall through to global.
- `secrets.env` files are loaded with `dotenvy`, never committed to git.
- When opening a cave, ensure its `.gitignore` includes `.granit/secrets.env`.
- `serde_yml` for YAML (de)serialization. `dirs` crate for platform-correct config paths.

### Markdown Processing

- `pulldown-cmark` parses markdown on the backend
- YAML frontmatter is stripped before rendering and parsed separately for metadata
- Wiki-links (`[[note-name]]`) are resolved to the matching `.md` file by filename
- Backend returns rendered HTML to the frontend for display

### Editor

Two modes toggled by the user:
- **Edit mode**: Raw markdown in a `<textarea>` (no styling)
- **Read mode**: Rendered HTML preview (read-only)

The long-term goal is an Obsidian-style live preview (WYSIWYM), but the architecture should support swapping the editor component later.

### AI Agent

Built with `rig-core` in the backend. Features:
- Side panel chat UI (similar to Copilot in VS Code)
- CRUD tools for cave operations (create, read, update, delete notes)
- In-memory vector database for RAG over cave contents
- Configurable LLM provider

Agent logic lives entirely in the backend. The frontend only renders the chat UI and streams responses via IPC.

## Development

### Build & Run

```sh
cd src-tauri && cargo tauri dev    # Full app (launches Trunk + Tauri)
trunk serve                        # Frontend only (port 1420)
cargo test -p granit               # Backend unit tests
cargo test -p granit-types         # Shared types tests
wasm-pack test --headless --firefox # Frontend WASM tests (includes text editing)
```
 
### Workflow

- **Branches**: Work on feature branches. Never commit directly to `main`.
- **Commits**: Follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- **Pull requests**: Use `gh pr create` (GitHub CLI) to open PRs.
- **After changes**: Always run `cargo fmt`, `cargo clippy`, and `cargo test` before committing.
- **Dependencies**: Use `cargo add <crate>` to add new dependencies (ensures latest version). Never hand-edit `Cargo.toml` dependency lines.
- **Planning**: Use the Bears task tracker skill for breaking down features into epics and sub-tasks.

### Conventions

- **Language**: All code, comments, and documentation in English.
- **Errors**: Define typed error enums with `thiserror`. No `anyhow`. Return `Result<T, MyError>` from commands.
- **Tauri commands**: One `#[tauri::command]` per operation. Keep handlers thin — delegate to modules.
- **Frontend**: Leptos reactive signals (`signal()`, `RwSignal`). Tailwind utility classes. Minimal JavaScript interop.
- **Testing**: Unit tests in the backend (`#[cfg(test)]` modules). Test cave operations, markdown parsing, and agent tools. No E2E tests yet.
- **Naming**: Snake_case for Rust. Kebab-case for filenames. Cave note filenames are user-controlled.
- **No over-engineering**: This is a personal tool. Build what's needed now. No plugin system, no sync, no abstractions for hypothetical features.

### Developer Experience

- The user is experienced with Rust backend development — keep backend explanations concise.
- The user is less experienced with frontend/Leptos/Tailwind — provide more guidance, examples, and explanations for frontend changes.

### Key Crates

| Crate | Purpose |
|-------|---------|
| `tauri` 2 | Desktop app framework, IPC, windowing |
| `leptos` 0.8 | Frontend UI (CSR/WASM) |
| `pulldown-cmark` | Markdown → HTML |
| `rig-core` | AI agent framework |
| `thiserror` | Typed error derivation |
| `serde` / `serde_json` | Serialization |
| `serde-wasm-bindgen` | Frontend ↔ JS value conversion |
| `serde_yml` | YAML config (de)serialization |
| `dirs` | Platform-correct config/data directories |
| `dotenvy` | Load `secrets.env` files |

### Release Process

Versions must stay in sync across three places:

| File | Field |
|------|-------|
| `Cargo.toml` (root) | `version` |
| `src-tauri/Cargo.toml` | `version` |
| `src-tauri/tauri.conf.json` | `version` |

The git tag **must** match the version in these files exactly (e.g. version `1.2.3` → tag `v1.2.3`).

Steps to cut a release:

```sh
# 1. Bump version in all three files to X.Y.Z — edit them, then verify:
grep -E '^version' Cargo.toml src-tauri/Cargo.toml
grep '"version"' src-tauri/tauri.conf.json

# 2. Commit the version bump
git add Cargo.toml src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to X.Y.Z"

# 3. Tag exactly matching the version
git tag vX.Y.Z

# 4. Push branch and tag
git push && git push --tags
```

Pushing the tag triggers `.github/workflows/release.yml`:
1. CI checks (fmt, clippy, tests) run first
2. On success, cross-platform Tauri builds run (macOS arm64/x86_64, Linux x86_64, Windows x86_64)
3. A **draft** GitHub Release is created with all artifacts attached
4. Review and publish the draft release on GitHub when ready

### Deferred Features (Not Yet — Don't Build)

- Full-text search (currently filename/title only)
- File watching / live reload on external changes
- Obsidian-style live preview editor
- Backlinks panel
- Sync (caves are local-only; user manages sync externally)
