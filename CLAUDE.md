# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Granit is

Granit is a minimal desktop note-taking app. It opens a local directory (a "cave") of markdown notes and provides a rendered reader, an explorer sidebar, and an integrated streaming AI agent. Built for personal use — no plugins, no sync, no speculative abstractions.

## Workspace layout

A Cargo workspace with three crates plus a JS build step:

- `granit-ui` (root crate, `src/`) — Leptos 0.8 CSR frontend, compiled to WASM. Thin IPC view layer.
- `granit` (`src-tauri/`) — Tauri 2 Rust backend. **Single source of truth** for state, file I/O, markdown rendering, cave operations, and agent behavior.
- `granit-types/` — shared types (config, document, agent, IPC payloads) used by both frontend and backend.
- `js/editor.ts` — CodeMirror 6 editor, bundled by esbuild to `build/codemirror.js` and exposed as `window.GranitEditor`.

## Build, test, run

```sh
npm ci                               # install JS deps (first time)
npm run build                        # build CSS (Tailwind) + CodeMirror bundle into build/
cd src-tauri && cargo tauri dev      # run the full app (launches Trunk + Tauri)
trunk serve                          # frontend only, port 1420

cargo test -p granit                 # backend tests
cargo test -p granit-types           # shared-types tests
cargo test -p granit <name>          # single backend test by name
wasm-pack test --headless --firefox  # frontend WASM tests

cargo fmt --all
cargo clippy -p granit --all-targets -- -D warnings              # backend lints
cargo clippy -p granit-ui --target wasm32-unknown-unknown -- -D warnings  # frontend lints
```

The frontend is a `wasm32-unknown-unknown` crate, so it must be clippy-checked against that target — a plain `cargo clippy --workspace` fails on it. CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)) runs fmt, both clippy invocations, `cargo test -p granit`, and the wasm-pack tests.

`npm run build` must run before the frontend can load — Trunk does not produce `build/styles.css` or `build/codemirror.js`. Use `npm run watch:css` / `npm run watch:js` during frontend iteration.

### Git hooks

Version-controlled hooks live in `.githooks/`. Activate them once per clone:

```sh
git config core.hooksPath .githooks
```

`pre-commit` runs `cargo fmt --all -- --check`; `pre-push` runs both clippy invocations and `cargo test -p granit`. They mirror CI so failures are caught before pushing.

## Architecture: backend-owned state

The backend is authoritative. The frontend holds reactive copies in `AppCtx` (see [src/app/context.rs](src/app/context.rs)) but never owns logic. **If the frontend needs derived data, add a backend command rather than duplicating logic in WASM.**

- **`AppState`** ([src-tauri/src/commands/state.rs](src-tauri/src/commands/state.rs)) is the `tauri::manage`d singleton. It holds the `AppConfig`, the open `Cave` (behind `SharedCave = Arc<Mutex<Option<Cave>>>`), the lazily-built `Agent`, an agent generation counter, and the RAG `CaveVectorIndex` — each behind its own mutex.
- **IPC** is one `#[tauri::command]` per operation, all registered in `tauri::generate_handler!` in [src-tauri/src/lib.rs](src-tauri/src/lib.rs). Handlers are thin and delegate into `cave/`, `agent/`, or `markdown/` modules. The frontend calls them through the typed `invoke_cmd` wrappers in [src/app/ipc.rs](src/app/ipc.rs).
- **Cave mutations** (including those triggered by agent tools) are picked up by the frontend via an event listener in [src/app/mod.rs](src/app/mod.rs) that refreshes notes/folders/active note. The active note slug is pushed back to the backend (`set_active_note`) so agent tools can see it.
- **Errors** use `thiserror` with one error enum per module (`CaveError`, `AgentError`, `ConfigError`). Do **not** use `anyhow`.

## Cave model

- A cave is any user-selected directory containing markdown notes plus a `.granit/` directory (`config.yml` + `templates/`).
- **Filenames are globally unique across the whole cave** — two notes in different subfolders cannot share a name. The filename stem is the note's identity, slug, and displayed title. Frontmatter does **not** override the title.
- Wiki-links `[[note]]` / `[[note|label]]` resolve by filename across the whole cave, not by relative path. Broken links are styled separately.
- **Heading anchors**: a heading marked with a pandoc attribute — `# Volvo {#volvo}` — becomes a wiki-link target in the *same global namespace as note filenames*, linkable with plain `[[Volvo]]` (resolves to `note#anchor`, scrolling the reader to the heading). Plain headings without `{#id}` are not targets. Anchor ids must be globally unique against both note slugs and other anchors; a duplicate refuses to open the cave (`CaveError::DuplicateAnchor`). The anchor index lives on `Cave` alongside `backlinks`; `Cave::resolve_link` is the resolver passed to markdown rendering.
- `Cave` keeps in-memory indexes (slug→path, backlinks, templates) populated lazily via `ensure_scanned()`. Use `AppState::with_cave` / `with_shared_cave`, which lock, ensure-scan, then run a closure.
- Config is **cave-local**: each cave stores its own sidebar/theme/font/daily-note/agent settings in `.granit/config.yml`. The last-open cave path is persisted separately via `tauri-plugin-store`. `active_cave` is runtime-only in IPC responses and is **never** serialized into cave YAML.
- Hidden directories and `.granit/` are excluded from note scans. Templates live in `.granit/templates/` with their own flat slug namespace.

## Markdown

Rendered in the backend with `pulldown-cmark` ([src-tauri/src/markdown/](src-tauri/src/markdown/)). Frontmatter (YAML: tags, timestamps, icon, `favorite`) is parsed separately from the body. Raw HTML is sanitized before reaching the webview. Reader mode renders backend HTML; edit mode uses CodeMirror. Task-list checkboxes are interactive in the reader and disabled in agent-rendered markdown.

## AI agent

All agent logic is backend-side ([src-tauri/src/agent/](src-tauri/src/agent/)), built on `rig-core`.

- **Providers**: Ollama, Anthropic, Mistral, and any OpenAI/ChatGPT-compatible endpoint (custom base URL + API key, built on rig's `openai` client). `ProviderAgent` is an enum over the four `rig` agent types; the `provider_dispatch!` / `provider_map!` macros fan one expression across all variants. Add a provider by extending the enum, the macros, a `build_*` constructor, and `ProviderConfig`.
- The `Agent` is built lazily by `AppState::ensure_agent` and torn down (`reset_agent`, bumping the generation counter) whenever config/provider/model/mode changes mid-stream.
- **Tools** ([src-tauri/src/agent/tools/](src-tauri/src/agent/tools/)) cover notes, folders, templates, daily notes, todos, search, web fetch, and web search. `build_toolset` filters out `disabled_tools` from config.
- **Modes**: `Ask` and `Agent` (`AgentMode`). RAG context is injected only in `Ask` mode.
- **RAG / vector index** ([src-tauri/src/agent/vectordb.rs](src-tauri/src/agent/vectordb.rs)): `CaveVectorIndex` embeds notes with local CPU `fastembed`, caches embeddings to `.granit/embeddings.bin` (via `rkyv`), and is wired into the agent builder as `dynamic_context(rag_top_n, index)`. The index is built/rebuilt in a background task when a cave opens or RAG config changes ([src-tauri/src/commands/config.rs](src-tauri/src/commands/config.rs)), and incrementally updated on note create/save/rename/delete ([src-tauri/src/commands/cave.rs](src-tauri/src/commands/cave.rs)).
- Streaming: `Agent::stream_with_history` returns a provider-erased `AgentStream` of `AgentStreamItem`s (text / tool call / tool result / done).

## Frontend conventions

- Leptos signals + the `AppCtx` context (`expect_context::<AppCtx>()`) rather than threading props. IPC goes through [src/app/ipc.rs](src/app/ipc.rs) wrappers.
- Styling is Tailwind CSS 4 + DaisyUI 5 utility classes written directly in `view!` macros. Prefer DaisyUI component classes before hand-rolling.
- Icons use `leptos_icons` + `icondata_lu`. `Icon` has **no** `class` prop — put sizing/color/spacing/rotation on a wrapper element. Provider brand assets live in `ProviderIcon` ([src/app/components/icons.rs](src/app/components/icons.rs)).

## Conventions

- All code, comments, and docs in English.
- Build only what is needed now. No plugin system, sync layer, or speculative abstractions (see deferred: file watching / external reload, live-preview editor, backlinks panel, sync).
- The repo owner is strong on Rust backend work and less experienced with Leptos/Tailwind/DaisyUI — be more explicit and cautious for frontend changes.
- Before committing, run `cargo fmt`, `cargo clippy`, and the relevant tests.
